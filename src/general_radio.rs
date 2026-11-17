//! 委任部隊の命令や戦況の変化を、将軍ごとの通信として通知する。

use crate::general_dialogue;
use crate::generals::GENERALS;
use crate::map::{tile_of, TS};
use crate::sim::{dist, Order, Phase, Target, World, CAPTURE_R};
use std::cmp::Reverse;

pub const EXPRESSIONS: usize = 10;
pub const EXPRESSION_SIZE: u32 = 96;
const CELL_BYTES: usize = (EXPRESSION_SIZE * EXPRESSION_SIZE * 4) as usize;
const ART: &[u8; GENERALS.len() * EXPRESSIONS * CELL_BYTES] =
    include_bytes!("../assets/generals/expressions/expressions.bgra");
const MAX_WAITING: usize = 12;

// 左から右、上から下の順。肖像シートと台詞集で共通。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Ready,
    Advance,
    Hold,
    Contact,
    Danger,
    Support,
    Loss,
    Relief,
    Capture,
    Victory,
}

impl Cue {
    fn priority(self) -> u8 {
        match self {
            Self::Support | Self::Victory => 3,
            Self::Danger | Self::Loss | Self::Capture => 2,
            _ => 1,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Report {
    pub general: usize,
    pub cue: Cue,
    pub tile: (i32, i32),
    variant: usize,
    created: f32,
}

impl Report {
    pub fn line(self) -> &'static str {
        general_dialogue::line(self.general, self.cue as usize, self.variant)
    }
}

pub fn expression_pixels(general: usize, cue: Cue) -> Vec<u32> {
    let start = (general * EXPRESSIONS + cue as usize) * CELL_BYTES;
    ART[start..start + CELL_BYTES]
        .chunks_exact(4)
        .map(|p| u32::from_le_bytes([p[0], p[1], p[2], p[3]]))
        .collect()
}

#[derive(Clone, Copy, Default)]
struct Status {
    alive: usize,
    advancing: bool,
    contact: bool,
    danger: bool,
    capturing: bool,
    tile: (i32, i32),
}

#[derive(Clone, Copy, Default)]
struct Group {
    status: Status,
    sum: (f32, f32),
    /// 最も急を要する車両の状態と位置。
    aid_at: Option<((bool, usize), (i32, i32))>,
}

#[derive(Default)]
pub struct Radio {
    last: [Status; GENERALS.len()],
    aid_after: [f32; GENERALS.len()],
    variants: [u16; GENERALS.len()],
    pending: Vec<Report>,
    current: Option<(Report, f32)>,
    next_check: f32,
    finished: bool,
}

impl Radio {
    pub fn current(&self) -> Option<Report> {
        self.current.map(|(report, _)| report)
    }

    pub fn update(&mut self, world: &World, dt: f32, paused: bool) {
        if world.phase == Phase::Prep {
            return;
        }
        if world.phase == Phase::Over && !self.finished {
            self.finish(world);
        } else if world.phase == Phase::Battle && world.time >= self.next_check && !paused {
            self.next_check = world.time + 1.0;
            self.observe(world);
        }
        if paused {
            return;
        }
        if let Some((_, remaining)) = &mut self.current {
            *remaining -= dt;
            if *remaining <= 0.0 {
                self.current = None;
            }
        }
        self.pending
            .retain(|report| world.time - report.created < 36.0);
        let best = self
            .pending
            .iter()
            .enumerate()
            .max_by_key(|&(i, report)| (report.cue.priority(), Reverse(i)));
        if let Some((i, next)) = best {
            if self.current.is_none_or(|(shown, _)| {
                next.cue == Cue::Support && shown.cue.priority() < next.cue.priority()
            }) {
                self.current = Some((self.pending.remove(i), 5.5));
            }
        }
    }

    fn observe(&mut self, world: &World) {
        if !world.units.iter().any(|unit| unit.general.is_some()) {
            return;
        }
        let visible: Vec<_> = world
            .units
            .iter()
            .enumerate()
            .filter(|(i, _)| world.targetable_by(0, *i))
            .map(|(_, unit)| unit.pos)
            .collect();
        let enemy_base = world.base_pos(1);
        let mut groups = [Group::default(); GENERALS.len()];
        for unit in world.units.iter().filter(|unit| unit.alive) {
            let Some(id) = unit.general else { continue };
            let group = &mut groups[id];
            let threats = visible
                .iter()
                .filter(|&&pos| dist(pos, unit.pos) < 9.0 * TS)
                .count();
            group.status.alive += 1;
            group.status.advancing |=
                matches!(unit.order, Order::Move { goal, .. } if goal.0 >= unit.pos.0 - TS);
            group.status.contact |= threats != 0
                || matches!(unit.target, Some(Target::Unit(_)))
                || matches!(unit.order, Order::Attack(Target::Unit(_)));
            let health = unit.hp / unit.spec().hp;
            group.status.danger |= threats > 1 || (threats != 0 && health < 0.65);
            let risk = (threats != 0 && health < 0.40, threats);
            if group.aid_at.is_none_or(|(old, _)| risk > old) {
                group.aid_at = Some((risk, unit.tile()));
            }
            group.status.capturing |=
                world.capture[1] > 0.0 && dist(unit.pos, enemy_base) <= CAPTURE_R * TS;
            group.sum.0 += unit.pos.0;
            group.sum.1 += unit.pos.1;
        }
        for (id, mut group) in groups.into_iter().enumerate() {
            let prior = self.last[id];
            let status = &mut group.status;
            if status.alive == 0 && prior.alive == 0 {
                continue;
            }
            let alive = status.alive as f32;
            status.tile = if status.alive == 0 {
                prior.tile
            } else {
                tile_of((group.sum.0 / alive, group.sum.1 / alive))
            };
            let aid = group.aid_at.and_then(|((critical, threats), tile)| {
                (critical
                    || (status.alive < prior.alive && threats >= status.alive)
                    || threats > status.alive + 1)
                    .then_some(tile)
            });
            let events = [
                (Cue::Ready, prior.alive == 0),
                (Cue::Advance, status.advancing && !prior.advancing),
                (
                    Cue::Hold,
                    status.alive > 0
                        && !status.advancing
                        && !status.contact
                        && (prior.advancing || prior.alive == 0),
                ),
                (Cue::Contact, status.contact && !prior.contact),
                (Cue::Danger, status.danger && !prior.danger && aid.is_none()),
                (Cue::Loss, status.alive < prior.alive),
                (
                    Cue::Relief,
                    prior.contact && !status.contact && status.alive > 0,
                ),
                (Cue::Capture, status.capturing && !prior.capturing),
            ];
            for (cue, triggered) in events {
                if triggered {
                    self.queue(id, cue, status.tile, world.time);
                }
            }
            if let Some(tile) = aid {
                if world.time >= self.aid_after[id] {
                    self.aid_after[id] = world.time + 30.0;
                    self.queue(id, Cue::Support, tile, world.time);
                }
            } else {
                self.pending
                    .retain(|report| report.general != id || report.cue != Cue::Support);
            }
            self.last[id] = *status;
        }
    }

    fn finish(&mut self, world: &World) {
        self.finished = true;
        self.pending.clear();
        self.current = None;
        let Some(winner) = world.winner else { return };
        let speaker = world
            .units
            .iter()
            .filter(|unit| winner == 0 || !unit.alive)
            .filter_map(|unit| unit.general.map(|id| (unit.alive, id, unit.tile())))
            .max_by_key(|&(alive, id, _)| (alive, Reverse(id)));
        if let Some((_, id, tile)) = speaker {
            let cue = if winner == 0 { Cue::Victory } else { Cue::Loss };
            self.queue(id, cue, tile, world.time);
        }
    }

    fn queue(&mut self, general: usize, cue: Cue, tile: (i32, i32), created: f32) {
        if self
            .pending
            .iter()
            .any(|r| r.general == general && r.cue == cue)
        {
            return;
        }
        if self.pending.len() >= MAX_WAITING {
            let Some((i, lowest)) = self
                .pending
                .iter()
                .enumerate()
                .min_by_key(|&(i, r)| (r.cue.priority(), Reverse(i)))
            else {
                return;
            };
            if lowest.cue.priority() >= cue.priority() {
                return;
            }
            self.pending.remove(i);
        }
        let mask = 1 << cue as usize;
        let variant = usize::from(self.variants[general] & mask != 0);
        self.variants[general] ^= mask;
        self.pending.push(Report {
            general,
            cue,
            tile,
            variant,
            created,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Nation, Terrain};
    use crate::map::{center, Field};
    use crate::sim::Kind;

    #[test]
    fn rescue_preempts_a_routine_report_and_points_at_the_damaged_tank() {
        let mut world = World::new(Field::Kursk, 13, 3000);
        world.map.t.fill(Terrain::Plain);
        let damaged = (13, 25);
        world.units.push(World::new_unit(
            0,
            Kind::Tank,
            Nation::Germany,
            0,
            center((8, 25)),
        ));
        world.units.push(World::new_unit(
            0,
            Kind::Tank,
            Nation::Germany,
            0,
            center(damaged),
        ));
        world.units.push(World::new_unit(
            1,
            Kind::Tank,
            Nation::Germany,
            0,
            center((70, 35)),
        ));
        assert!(world.assign_general(0, Some(0)));
        assert!(world.assign_general(1, Some(0)));
        world.start_battle();
        world.order(
            0,
            Order::Move {
                goal: center((50, 25)),
                attack: true,
            },
        );

        let mut radio = Radio::default();
        radio.update(&world, 0.0, false);
        assert_eq!(radio.current().unwrap().cue, Cue::Ready);
        radio.update(&world, 5.5, false);
        assert_eq!(radio.current().unwrap().cue, Cue::Advance);

        world.units[1].hp = 1.0;
        world.units[2].pos = center((17, 25));
        world.spotted[0][2] = true;
        world.time = 2.0;
        radio.update(&world, 0.0, false);
        let report = radio.current().unwrap();
        assert_eq!(report.cue, Cue::Support);
        assert_eq!(report.tile, damaged);
        assert_eq!(report.variant, 0);

        world.time = 33.0;
        radio.update(&world, 5.5, false);
        assert_eq!(radio.current().unwrap().cue, Cue::Support);
        assert_eq!(radio.current().unwrap().variant, 1);
        world.winner = Some(0);
        world.phase = Phase::Over;
        radio.update(&world, 0.0, false);
        assert_eq!(radio.current().unwrap().cue, Cue::Victory);
    }
}
