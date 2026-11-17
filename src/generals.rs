//! 将軍の名簿と、委任された自軍戦車の自動指揮。

use crate::data::Terrain;
use crate::map::{center, idx, inside, tile_of, MH, MW, TS};
use crate::sim::{dist, Kind, Order, Phase, Target, World, CAPTURE_R};

type Point = (f32, f32);

#[derive(Clone, Copy)]
pub enum Strategy {
    Offensive,
    Guerrilla,
    Defense,
    Raid,
}

impl Strategy {
    pub fn name(self) -> &'static str {
        match self {
            Self::Offensive => t!("積極攻勢", "Aggressive advance", "积极攻势"),
            Self::Guerrilla => t!("ゲリラ抵抗", "Guerrilla resistance", "游击抵抗"),
            Self::Defense => t!("拠点防衛", "Base defense", "据点防御"),
            Self::Raid => t!("遊撃", "Mobile raiding", "机动突袭"),
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Offensive => t!(
                "敵を攻撃しながら敵基地へ前進。",
                "Engages nearby enemies and advances on their base.",
                "攻击附近敌军，向敌方基地推进。"
            ),
            Self::Guerrilla => t!(
                "遮蔽物に待ち伏せし、損傷や集中攻撃で後退。",
                "Ambushes from cover and falls back when damaged or outnumbered.",
                "依托掩体伏击，受损或寡不敌众时后撤。"
            ),
            Self::Defense => t!(
                "自軍基地の周辺で迎撃し、離れた敵は深追いしない。",
                "Intercepts threats near your base and stays within its perimeter.",
                "拦截基地周边威胁，坚守防区。"
            ),
            Self::Raid => t!(
                "側面を巡回し、孤立した弱い敵や損傷車を狙う。",
                "Patrols a flank and strikes vulnerable or damaged enemies.",
                "沿侧翼巡逻，攻击薄弱或受损的敌军。"
            ),
        }
    }

    pub fn color(self) -> [f32; 4] {
        match self {
            Self::Offensive => [1.0, 0.61, 0.42, 1.0],
            Self::Guerrilla => [0.65, 0.86, 0.48, 1.0],
            Self::Defense => [0.58, 0.80, 1.0, 1.0],
            Self::Raid => [0.86, 0.66, 1.0, 1.0],
        }
    }
}

pub struct General {
    names: [&'static str; 3],
    pub strategy: Strategy,
}

impl General {
    pub fn name(&self) -> &'static str {
        t!(self.names[0], self.names[1], self.names[2])
    }
}

use Strategy::{Defense, Guerrilla, Offensive, Raid};

// 肖像データの並びは tools/make_generals.py と対応する。
pub const GENERALS: [General; 16] = [
    General {
        names: ["朝霧 凛", "Rin Asagiri", "朝雾 凛"],
        strategy: Offensive,
    },
    General {
        names: ["水野 結衣", "Yui Mizuno", "水野 结衣"],
        strategy: Guerrilla,
    },
    General {
        names: ["藤森 栞", "Shiori Fujimori", "藤森 栞"],
        strategy: Defense,
    },
    General {
        names: ["九条 茜", "Akane Kujo", "九条 茜"],
        strategy: Offensive,
    },
    General {
        names: ["橘 玲", "Rei Tachibana", "橘 玲"],
        strategy: Raid,
    },
    General {
        names: ["如月 澪", "Mio Kisaragi", "如月 澪"],
        strategy: Guerrilla,
    },
    General {
        names: ["レオン・フォーゲル", "Leon Vogel", "莱昂·福格尔"],
        strategy: Offensive,
    },
    General {
        names: ["ミハイル・イワノフ", "Mikhail Ivanov", "米哈伊尔·伊万诺夫"],
        strategy: Defense,
    },
    General {
        names: ["ジュリアン・アーデン", "Julian Arden", "朱利安·阿登"],
        strategy: Raid,
    },
    General {
        names: ["黒田 海", "Kai Kuroda", "黑田 海"],
        strategy: Guerrilla,
    },
    General {
        names: ["芝 大輔", "Daisuke Shiba", "芝 大辅"],
        strategy: Defense,
    },
    General {
        names: ["加賀美 直人", "Naoto Kagami", "加贺美 直人"],
        strategy: Raid,
    },
    General {
        names: ["桐原 星奈", "Sena Kirihara", "桐原 星奈"],
        strategy: Raid,
    },
    General {
        names: ["ルーカス・レン", "Lucas Wren", "卢卡斯·伦"],
        strategy: Defense,
    },
    General {
        names: ["高峰 いすず", "Isuzu Takamine", "高峰 五十铃"],
        strategy: Guerrilla,
    },
    General {
        names: ["エミール・ファルク", "Emil Falk", "埃米尔·法尔克"],
        strategy: Offensive,
    },
];

pub const PORTRAIT_SIZE: u32 = 128;
const PORTRAIT_BYTES: usize = (PORTRAIT_SIZE * PORTRAIT_SIZE * 4) as usize;
const PORTRAITS: &[u8; PORTRAIT_BYTES * GENERALS.len()] =
    include_bytes!("../assets/generals/portraits.bgra");

pub fn portrait_pixels(id: usize) -> Vec<u32> {
    PORTRAITS[id * PORTRAIT_BYTES..(id + 1) * PORTRAIT_BYTES]
        .chunks_exact(4)
        .map(|p| u32::from_le_bytes([p[0], p[1], p[2], p[3]]))
        .collect()
}

#[derive(Default)]
pub struct GeneralCommand {
    next_update: f32,
    plans: Vec<Option<Plan>>,
}

enum Plan {
    Advance(Point),
    Ambush { cover: Point, rear: Point },
    Guard(Point),
    Patrol { points: [Point; 3], leg: usize },
}

impl GeneralCommand {
    pub fn update(&mut self, world: &mut World) {
        if world.phase != Phase::Battle || world.time < self.next_update {
            return;
        }
        self.next_update = world.time + 1.4;
        if !world
            .units
            .iter()
            .any(|unit| unit.alive && unit.general.is_some())
        {
            return;
        }
        if self.plans.is_empty() {
            self.prepare(world);
        }
        let visible: Vec<_> = (0..world.units.len())
            .filter(|&i| world.targetable_by(0, i))
            .collect();
        for (i, plan) in self.plans.iter_mut().enumerate() {
            let Some(plan) = plan.as_mut() else { continue };
            let unit = &world.units[i];
            if !unit.alive {
                continue;
            }
            let order = plan.order(world, i, &visible);
            let same = match (unit.order, order) {
                (
                    Order::Move {
                        goal: old,
                        attack: a,
                    },
                    Order::Move {
                        goal: new,
                        attack: b,
                    },
                ) => a == b && dist(old, new) < TS,
                (old, new) => old == new,
            };
            if !same {
                world.order(i, order);
            }
        }
    }

    fn prepare(&mut self, world: &World) {
        self.plans.resize_with(world.units.len(), || None);
        let home = world.map.bases[0];
        let enemy = world.map.bases[1];
        let reachable = world.map.flood(home);
        for (i, unit) in world.units.iter().enumerate() {
            let Some(id) = unit.general else { continue };
            let lane = unit.tile().1;
            let lateral = (i % 5) as i32 - 2;
            let point =
                |goal, radius, cover| best_tile(world, &reachable, unit.pos, goal, radius, cover);
            self.plans[i] = Some(match GENERALS[id].strategy {
                Offensive => Plan::Advance(point((enemy.0 - 1, enemy.1 + lateral), 2, false)),
                Guerrilla => Plan::Ambush {
                    cover: point((MW / 2 - 8, lane), 7, true),
                    rear: point((MW / 3 - 6, lane), 7, true),
                },
                Defense => Plan::Guard(point((home.0 + 3, home.1 + lateral), 3, false)),
                Raid => {
                    let flank = if id % 2 == 0 { MH / 5 } else { MH * 4 / 5 };
                    let flank = flank + lateral;
                    Plan::Patrol {
                        points: [
                            point((MW / 2 - 5, flank), 9, false),
                            point((MW * 3 / 4, flank), 9, false),
                            point((MW * 3 / 4, (flank + enemy.1) / 2), 7, false),
                        ],
                        leg: 0,
                    }
                }
            });
        }
    }
}

fn best_tile(
    world: &World,
    reachable: &[bool],
    fallback: Point,
    goal: (i32, i32),
    radius: i32,
    prefer_cover: bool,
) -> Point {
    let mut best = None;
    for y in (goal.1 - radius)..=(goal.1 + radius) {
        for x in (goal.0 - radius)..=(goal.0 + radius) {
            if !inside(x, y) || !reachable[idx(x, y)] || world.teeth_at[idx(x, y)] != usize::MAX {
                continue;
            }
            let terrain = world.map.at(x, y);
            let covered = matches!(terrain, Terrain::Forest | Terrain::Town | Terrain::Bocage);
            let score = (x - goal.0).pow(2) + (y - goal.1).pow(2)
                - if prefer_cover && covered { 50 } else { 0 };
            if best.is_none_or(|(value, _)| score < value) {
                best = Some((score, (x, y)));
            }
        }
    }
    best.map_or(fallback, |(_, tile)| center(tile))
}

impl Plan {
    fn order(&mut self, world: &World, index: usize, visible: &[usize]) -> Order {
        let unit = &world.units[index];
        let position = unit.pos;
        let nearest = |origin: Point, radius: f32| {
            visible
                .iter()
                .copied()
                .filter_map(|i| {
                    let d = dist(origin, world.units[i].pos);
                    (d <= radius).then_some((i, d))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i)
        };
        let go = |goal, attack| {
            if dist(position, goal) < TS * 0.62 {
                Order::Idle
            } else {
                Order::Move { goal, attack }
            }
        };
        match self {
            Self::Advance(goal) => nearest(position, TS * 15.0)
                .map_or(go(*goal, true), |i| Order::Attack(Target::Unit(i))),
            Self::Ambush { cover, rear } => {
                let pressure = visible
                    .iter()
                    .filter(|&&i| dist(position, world.units[i].pos) < TS * 5.0)
                    .count();
                if unit.hp < unit.spec().hp * 0.42 || pressure >= 2 {
                    go(*rear, false)
                } else {
                    go(*cover, true)
                }
            }
            Self::Guard(goal) => {
                let base = world.base_pos(0);
                if let Some(i) = nearest(base, TS * (CAPTURE_R + 6.0)) {
                    go(center(tile_of(world.units[i].pos)), true)
                } else {
                    go(*goal, false)
                }
            }
            Self::Patrol { points, leg } => {
                let prey = visible
                    .iter()
                    .copied()
                    .filter_map(|i| {
                        let enemy = &world.units[i];
                        let d = dist(position, enemy.pos);
                        if d >= TS * unit.spec().range.min(11.0) {
                            return None;
                        }
                        let hp = enemy.hp / enemy.spec().hp;
                        let isolated = || {
                            !visible.iter().any(|&other| {
                                other != i
                                    && world.units[other].kind == Kind::Tank
                                    && dist(enemy.pos, world.units[other].pos) < TS * 4.0
                            })
                        };
                        let vulnerable = enemy.kind == Kind::Decoy
                            || hp < 0.65
                            || (enemy.spec().side < unit.spec().pen * 0.8 && isolated());
                        vulnerable.then_some((i, d / TS + hp * 4.0))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((i, _)) = prey {
                    return Order::Attack(Target::Unit(i));
                }
                if dist(position, points[*leg]) < TS {
                    *leg = (*leg + 1) % points.len();
                }
                go(points[*leg], false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Nation;
    use crate::map::Field;

    fn tank(world: &mut World, side: usize, kind: Kind, tile: (i32, i32)) -> usize {
        let index = world.units.len();
        world.units.push(World::new_unit(
            side,
            kind,
            Nation::Germany,
            0,
            center(tile),
        ));
        index
    }

    #[test]
    fn assignment_only_accepts_friendly_tanks_during_deployment() {
        let mut world = World::new(Field::Kursk, 19, 3000);
        let own = tank(&mut world, 0, Kind::Tank, (8, 15));
        let decoy = tank(&mut world, 0, Kind::Decoy, (9, 15));
        let enemy = tank(&mut world, 1, Kind::Tank, (69, 15));
        assert!(!world.assign_general(decoy, Some(0)));
        assert!(!world.assign_general(enemy, Some(0)));
        assert!(!world.assign_general(own, Some(GENERALS.len())));
        assert!(world.assign_general(own, Some(0)));
        assert!(!world.units[own].manually_controlled());
        assert!(world.assign_general(own, None));
        assert!(world.units[own].manually_controlled());
        assert!(world.assign_general(own, Some(2)));
        world.start_battle();
        assert!(!world.assign_general(own, None));
        assert_eq!(world.units[own].general, Some(2));
    }

    #[test]
    fn strategies_issue_distinct_orders_and_leave_direct_control_alone() {
        let mut world = World::new(Field::Kursk, 37, 3000);
        world.map.t.fill(Terrain::Plain);
        let lane = 26;
        let ambush_tile = (MW / 2 - 7, lane);
        world.map.t[idx(ambush_tile.0, ambush_tile.1)] = Terrain::Forest;
        let advance = tank(&mut world, 0, Kind::Tank, (12, lane - 1));
        let guerrilla = tank(&mut world, 0, Kind::Tank, (12, lane));
        let guard = tank(&mut world, 0, Kind::Tank, (12, lane + 1));
        let raid = tank(&mut world, 0, Kind::Tank, (12, lane + 2));
        let direct = tank(&mut world, 0, Kind::Tank, (12, lane + 3));
        for (unit, general) in [(advance, 0), (guerrilla, 1), (guard, 2), (raid, 4)] {
            assert!(world.assign_general(unit, Some(general)));
        }
        world.start_battle();
        let mut command = GeneralCommand::default();
        command.update(&mut world);
        assert!(
            matches!(world.units[advance].order, Order::Move { goal, attack: true } if tile_of(goal).0 > MW * 3 / 4)
        );
        assert_eq!(
            world.units[guerrilla].order,
            Order::Move {
                goal: center(ambush_tile),
                attack: true
            }
        );
        assert!(
            matches!(world.units[guard].order, Order::Move { goal, attack: false } if tile_of(goal).0 < 12)
        );
        assert!(
            matches!(world.units[raid].order, Order::Move { goal, attack: false } if tile_of(goal).1 < MH / 3)
        );
        assert_eq!(world.units[direct].order, Order::Idle);

        world.units[guerrilla].hp = 1.0;
        world.time = 2.0;
        command.update(&mut world);
        assert!(
            matches!(world.units[guerrilla].order, Order::Move { goal, attack: false } if tile_of(goal).0 < ambush_tile.0)
        );
    }
}
