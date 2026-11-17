//! 画面と操作：タイトル（戦場・予算・難易度の選択）、準備（購入と配置）、戦闘、結果
//!
//! 世界（地図・戦車）は WebGL / Direct3D の三角形で、文字と画面部品は Canvas2D / Direct2D で描く。

use crate::ai::{self, Commander};
use crate::data::{Budget, Nation, TankSpec, Weight, BUDGETS, DECOY_COST, NATIONS, TEETH_COST};
use crate::gfx::{Font, Gfx, Texture};

use crate::lang;
use crate::map::{center, idx, in_zone, tile_of, Field, FIELDS, MH, MW, TS};
use crate::sim::{dist, EndReason, FxKind, Kind, Order, Phase, Target, Unit, World, CAPTURE_R, DECOY_REVEAL, DRONE_SIGHT, DT, TIME_LIMIT};
use crate::Ev;

type C = [f32; 4];

const TEXT: C = [0.94, 0.91, 0.82, 1.0];
const DIM: C = [0.70, 0.68, 0.60, 1.0];
const GOLD: C = [0.98, 0.82, 0.42, 1.0];
const PANEL: C = [0.07, 0.08, 0.06, 0.90];
const EDGE: C = [0.55, 0.50, 0.34, 1.0];
const BLUE: C = [0.35, 0.62, 1.0, 1.0];
const RED: C = [1.0, 0.36, 0.30, 1.0];
const GOOD: C = [0.55, 0.92, 0.55, 1.0];

fn side_color(side: usize) -> C {
    if side == 0 {
        BLUE
    } else {
        RED
    }
}

fn with_a(c: C, a: f32) -> C {
    [c[0], c[1], c[2], c[3] * a]
}

fn shade(c: C, k: f32) -> C {
    [c[0] * k, c[1] * k, c[2] * k, c[3]]
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Title,
    Battle,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tool {
    Tank(Nation, usize),
    Decoy,
    Teeth,
    Remove,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Act {
    Field(usize),
    Budget(usize),
    Diff(usize),
    Reroll,
    Lang,
    Fullscreen,
    Start,
    Tab(usize),
    Tool(Tool),
    Auto,
    Clear,
    Begin,
    Pause,
    Speed(u32),
    Help,
    Menu,
    AttackMode,
    Stop,
    Drone,
    AllTanks,
    Again,
    ToTitle,
}

struct Btn {
    r: [f32; 4],
    act: Act,
}

struct LogLine {
    text: String,
    age: f32,
    good: bool,
}

pub struct Game {
    gfx: Gfx,
    screen: Screen,
    field_i: usize,
    budget_i: usize,
    diff_i: usize,
    seed: u32,
    world: World,
    commander: Commander,
    terrain: Option<Texture>,
    terrain_key: (Field, u32),
    fog: Option<Texture>,
    fog_px: Vec<u32>,
    cam: (f32, f32),
    zoom: f32,
    mouse: (f32, f32),
    ldown: Option<(f32, f32)>,
    mdown: Option<((f32, f32), (f32, f32))>,
    painting: bool,
    sel: Vec<usize>,
    buttons: Vec<Btn>,
    panels: Vec<[f32; 4]>,
    minimap: [f32; 4],
    tool: Tool,
    tab: usize,
    mimic: (Nation, usize),
    paused: bool,
    speed: u32,
    acc: f32,
    attack_mode: bool,
    log: Vec<LogLine>,
    show_help: bool,
    touch: bool,
    clock: f32,
    keys: [bool; 4],
    marker: Option<((f32, f32), f32, bool)>,
    menu_confirm: f32,
    notice: Option<(String, f32)>,
    /// ミニマップに出す出来事の場所
    pings: Vec<((f32, f32), f32)>,
}

fn diff_name(i: usize) -> &'static str {
    match i {
        0 => t!("易しい", "Easy", "简单"),
        1 => t!("普通", "Normal", "普通"),
        _ => t!("難しい", "Hard", "困难"),
    }
}

fn weight_name(w: Weight) -> &'static str {
    match w {
        Weight::Light => t!("軽", "Light", "轻"),
        Weight::Medium => t!("中", "Medium", "中"),
        Weight::Heavy => t!("重", "Heavy", "重"),
    }
}

impl Game {
    pub fn new(gfx: Gfx) -> Game {
        let seed = crate::platform::random_u32() | 1;
        let world = World::new(Field::Kursk, seed, Budget::Medium.points());
        let commander = Commander::new(1, seed, 1.0);
        let mut g = Game {
            gfx,
            screen: Screen::Title,
            field_i: 0,
            budget_i: 1,
            diff_i: 1,
            seed,
            world,
            commander,
            terrain: None,
            terrain_key: (Field::Kursk, 0),
            fog: None,
            fog_px: vec![0; (MW * MH) as usize],
            cam: (MW as f32 * TS / 2.0, MH as f32 * TS / 2.0),
            zoom: 0.5,
            mouse: (0.0, 0.0),
            ldown: None,
            mdown: None,
            painting: false,
            sel: Vec::new(),
            buttons: Vec::new(),
            panels: Vec::new(),
            minimap: [0.0; 4],
            tool: Tool::Tank(Nation::Germany, 1),
            tab: 0,
            mimic: (Nation::Germany, 3),
            paused: false,
            speed: 1,
            acc: 0.0,
            attack_mode: false,
            log: Vec::new(),
            show_help: false,
            touch: false,
            clock: 0.0,
            keys: [false; 4],
            marker: None,
            menu_confirm: 0.0,
            notice: None,
            pings: Vec::new(),
        };
        g.ensure_terrain();
        g
    }

    #[cfg_attr(windows, allow(dead_code))]
    pub fn apply_loaded(&mut self, _data: &[u8]) {}

    pub fn notify(&mut self, s: &str) {
        self.notice = Some((s.to_string(), 4.0));
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn state_text(&self) -> String {
        format!("screen={:?} field={:?} phase={:?} t={:.0} units={}", self.screen, self.world.map.field, self.world.phase, self.world.time, self.world.units.len())
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn crash_bytes(&self) -> Option<Vec<u8>> {
        None
    }

    /// タッチした場所に何があるか：0=地図 1=画面部品 2=ミニマップ
    #[cfg_attr(windows, allow(dead_code))]
    pub fn hit_kind(&self, x: f32, y: f32) -> u32 {
        let (x, y) = (x / self.gfx.ui, y / self.gfx.ui);
        if self.screen == Screen::Title || self.show_help || self.world.phase == Phase::Over {
            return 1;
        }
        let inr = |r: &[f32; 4]| x >= r[0] && y >= r[1] && x <= r[2] && y <= r[3];
        if inr(&self.minimap) {
            return 2;
        }
        if self.buttons.iter().any(|b| inr(&b.r)) || self.panels.iter().any(inr) {
            return 1;
        }
        0
    }

    fn field(&self) -> Field {
        FIELDS[self.field_i]
    }

    /// 地形の絵を作り直す（戦場か種が変わったとき）
    fn ensure_terrain(&mut self) {
        let key = (self.world.map.field, self.world.map.seed);
        if self.terrain.is_some() && self.terrain_key == key {
            return;
        }
        let (w, h, px) = self.world.map.render_pixels();
        self.terrain = self.gfx.create_texture(w, h, &px).ok();
        self.terrain_key = key;
        if self.fog.is_none() {
            self.fog = self.gfx.create_texture(MW as u32, MH as u32, &self.fog_px).ok();
        }
    }

    /// タイトル画面の地図を選んだ戦場で作り直す
    fn regen_preview(&mut self) {
        self.world = World::new(self.field(), self.seed, BUDGETS[self.budget_i].points());
        self.ensure_terrain();
    }

    fn start_prep(&mut self) {
        let budget = BUDGETS[self.budget_i].points();
        self.world = World::new(self.field(), self.seed, budget);
        let (skill, aggr) = match self.diff_i {
            0 => (0.8, 0.7),
            1 => (1.0, 1.0),
            _ => (1.12, 1.3),
        };
        self.world.skill = [1.0, skill];
        ai::prepare(&mut self.world, 1, self.seed ^ 0x51ed);
        self.commander = Commander::new(1, self.seed ^ 0xc0, aggr);
        self.screen = Screen::Battle;
        self.sel.clear();
        self.log.clear();
        self.paused = false;
        self.speed = 1;
        self.acc = 0.0;
        self.show_help = false;
        self.tool = Tool::Tank(NATIONS[self.tab], 1);
        self.ensure_terrain();
        // 自陣の配置範囲を映す
        let b = self.world.base_pos(0);
        self.zoom = (self.gfx.width as f32 / (34.0 * TS)).clamp(0.3, 2.0);
        self.cam = (b.0 + 10.0 * TS, b.1);
        self.clamp_cam();
        self.push_log(t!("準備：予算の範囲で戦車を買って自陣に配置し、デコイと竜の歯を置く", "Preparation: buy tanks within budget and deploy them, then place decoys and dragon's teeth", "准备：在预算内购买坦克部署于己方阵地，并布置诱饵和龙牙").to_string(), true);
    }

    fn push_log(&mut self, text: String, good: bool) {
        self.log.push(LogLine { text, age: 0.0, good });
        if self.log.len() > 40 {
            self.log.remove(0);
        }
    }

    // ------------------------------------------------------------ 座標

    fn to_world(&self, p: (f32, f32)) -> (f32, f32) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        (self.cam.0 + (p.0 - w / 2.0) / self.zoom, self.cam.1 + (p.1 - h / 2.0) / self.zoom)
    }

    fn to_screen(&self, p: (f32, f32)) -> (f32, f32) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        ((p.0 - self.cam.0) * self.zoom + w / 2.0, (p.1 - self.cam.1) * self.zoom + h / 2.0)
    }

    fn min_zoom(&self) -> f32 {
        (self.gfx.width as f32 / (MW as f32 * TS)).min(self.gfx.height as f32 / (MH as f32 * TS)) * 0.9
    }

    fn clamp_cam(&mut self) {
        self.zoom = self.zoom.clamp(self.min_zoom(), 3.0);
        let (mw, mh) = (MW as f32 * TS, MH as f32 * TS);
        self.cam.0 = self.cam.0.clamp(0.0, mw);
        self.cam.1 = self.cam.1.clamp(0.0, mh);
    }

    // ------------------------------------------------------------ 1フレーム

    pub fn frame(&mut self, evs: &[Ev], dt: f32, focused: bool) -> bool {
        for n in crate::platform::take_notes() {
            self.notify(&n);
        }
        self.clock += dt;
        let ui = self.gfx.ui;
        for e in evs {
            let e = match *e {
                Ev::MouseMove(x, y) => Ev::MouseMove(x / ui, y / ui),
                Ev::Down(b, x, y) => Ev::Down(b, x / ui, y / ui),
                Ev::Up(b, x, y) => Ev::Up(b, x / ui, y / ui),
                o => o,
            };
            self.handle(e);
        }
        if self.screen == Screen::Battle {
            self.scroll(dt, focused);
            let running = !self.paused && !self.show_help;
            if running && self.world.phase != Phase::Over {
                self.acc += dt * self.speed as f32;
                let mut steps = 0;
                while self.acc >= DT && steps < 12 {
                    self.commander.update(&mut self.world, DT);
                    self.world.step();
                    self.acc -= DT;
                    steps += 1;
                }
                if steps >= 12 {
                    self.acc = 0.0;
                }
            } else if self.world.phase == Phase::Over {
                // 結果画面でも煙などは動かす
                self.world.step();
            }
            for e in std::mem::take(&mut self.world.events) {
                if let Some(p) = e.pos {
                    self.pings.push((p, 0.0));
                }
                self.push_log(e.text, e.side == 1);
            }
            for p in self.pings.iter_mut() {
                p.1 += dt;
            }
            self.pings.retain(|p| p.1 < 2.5);
            let w = &self.world;
            self.sel.retain(|&i| w.units.get(i).is_some_and(|u| u.alive && u.side == 0 && u.kind != Kind::Decoy));
            for l in self.log.iter_mut() {
                l.age += dt;
            }
        }
        if let Some((_, t)) = &mut self.notice {
            *t -= dt;
            if *t <= 0.0 {
                self.notice = None;
            }
        }
        if let Some(m) = &mut self.marker {
            m.1 -= dt;
            if m.1 <= 0.0 {
                self.marker = None;
            }
        }
        self.menu_confirm = (self.menu_confirm - dt).max(0.0);
        self.render();
        false
    }

    fn scroll(&mut self, dt: f32, focused: bool) {
        let sp = 900.0 / self.zoom * dt;
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let mut d = (0.0, 0.0);
        if self.keys[0] {
            d.0 -= sp;
        }
        if self.keys[1] {
            d.1 -= sp;
        }
        if self.keys[2] {
            d.0 += sp;
        }
        if self.keys[3] {
            d.1 += sp;
        }
        if focused && !self.touch && self.mdown.is_none() && self.ldown.is_none() {
            let m = self.mouse;
            let e = 4.0;
            if m.0 <= e {
                d.0 -= sp;
            } else if m.0 >= w - e {
                d.0 += sp;
            }
            if m.1 <= e {
                d.1 -= sp;
            } else if m.1 >= h - e {
                d.1 += sp;
            }
        }
        if d != (0.0, 0.0) {
            self.cam.0 += d.0;
            self.cam.1 += d.1;
            self.clamp_cam();
        }
    }

    // ------------------------------------------------------------ 入力

    fn button_at(&self, p: (f32, f32)) -> Option<Act> {
        self.buttons.iter().rev().find(|b| p.0 >= b.r[0] && p.0 <= b.r[2] && p.1 >= b.r[1] && p.1 <= b.r[3]).map(|b| b.act)
    }

    fn over_ui(&self, p: (f32, f32)) -> bool {
        let inr = |r: &[f32; 4]| p.0 >= r[0] && p.1 >= r[1] && p.0 <= r[2] && p.1 <= r[3];
        self.buttons.iter().any(|b| inr(&b.r)) || self.panels.iter().any(inr)
    }

    fn in_minimap(&self, p: (f32, f32)) -> bool {
        let r = self.minimap;
        self.screen == Screen::Battle && p.0 >= r[0] && p.0 <= r[2] && p.1 >= r[1] && p.1 <= r[3]
    }

    fn minimap_jump(&mut self, p: (f32, f32)) {
        let r = self.minimap;
        let fx = ((p.0 - r[0]) / (r[2] - r[0])).clamp(0.0, 1.0);
        let fy = ((p.1 - r[1]) / (r[3] - r[1])).clamp(0.0, 1.0);
        self.cam = (fx * MW as f32 * TS, fy * MH as f32 * TS);
        self.clamp_cam();
    }

    fn handle(&mut self, e: Ev) {
        match e {
            Ev::Resize(w, h) => {
                self.gfx.resize(w, h);
                self.clamp_cam();
            }
            Ev::Compact(c) => self.gfx.set_compact(c),
            Ev::Touch(t) => self.touch = t,
            Ev::Scroll(_) | Ev::Char(_) => {}
            Ev::MouseMove(x, y) => {
                self.mouse = (x, y);
                if let Some((m0, c0)) = self.mdown {
                    self.cam = (c0.0 - (x - m0.0) / self.zoom, c0.1 - (y - m0.1) / self.zoom);
                    self.clamp_cam();
                }
                if self.ldown.is_some() && self.in_minimap((x, y)) && self.ldown.is_some_and(|d| self.in_minimap(d)) {
                    self.minimap_jump((x, y));
                }
                if self.painting && self.world.phase == Phase::Prep {
                    let p = self.to_world((x, y));
                    self.world.place_teeth(0, p);
                }
            }
            Ev::Wheel(d) => {
                if self.screen != Screen::Battle {
                    return;
                }
                let before = self.to_world(self.mouse);
                self.zoom = (self.zoom * 1.15f32.powf(d)).clamp(self.min_zoom(), 3.0);
                let after = self.to_world(self.mouse);
                self.cam.0 += before.0 - after.0;
                self.cam.1 += before.1 - after.1;
                self.clamp_cam();
            }
            Ev::Down(b, x, y) => self.down(b, (x, y)),
            Ev::Up(b, x, y) => self.up(b, (x, y)),
            Ev::Key(vk, down) => self.key(vk, down),
        }
    }

    fn down(&mut self, b: u8, p: (f32, f32)) {
        self.mouse = p;
        if b == 2 {
            self.mdown = Some((p, self.cam));
            return;
        }
        if b == 0 {
            if let Some(a) = self.button_at(p) {
                self.act(a);
                return;
            }
            if self.screen != Screen::Battle || self.show_help || self.world.phase == Phase::Over {
                return;
            }
            if self.in_minimap(p) {
                self.ldown = Some(p);
                self.minimap_jump(p);
                return;
            }
            if self.over_ui(p) {
                return;
            }
            let wp = self.to_world(p);
            match self.world.phase {
                Phase::Prep => self.apply_tool(wp),
                Phase::Battle => {
                    if self.attack_mode && !self.sel.is_empty() {
                        self.command(wp, true);
                        self.attack_mode = false;
                    } else {
                        self.ldown = Some(p);
                    }
                }
                Phase::Over => {}
            }
        } else if b == 1 {
            if self.screen != Screen::Battle || self.show_help || (self.over_ui(p) && !self.in_minimap(p)) {
                return;
            }
            let wp = if self.in_minimap(p) {
                let r = self.minimap;
                (((p.0 - r[0]) / (r[2] - r[0])) * MW as f32 * TS, ((p.1 - r[1]) / (r[3] - r[1])) * MH as f32 * TS)
            } else {
                self.to_world(p)
            };
            match self.world.phase {
                Phase::Prep => {
                    self.world.remove_at(0, wp);
                }
                Phase::Battle => {
                    let am = self.attack_mode;
                    self.command(wp, am);
                    self.attack_mode = false;
                }
                Phase::Over => {}
            }
        }
    }

    fn up(&mut self, b: u8, p: (f32, f32)) {
        self.mouse = p;
        if b == 2 {
            self.mdown = None;
            return;
        }
        if b != 0 {
            return;
        }
        self.painting = false;
        let Some(d) = self.ldown.take() else { return };
        if self.in_minimap(d) {
            return;
        }
        if self.world.phase != Phase::Battle {
            return;
        }
        let drag = (p.0 - d.0).abs() > 8.0 || (p.1 - d.1).abs() > 8.0;
        if drag {
            // 範囲選択
            let (a, c) = (self.to_world(d), self.to_world(p));
            let (x0, x1) = (a.0.min(c.0), a.0.max(c.0));
            let (y0, y1) = (a.1.min(c.1), a.1.max(c.1));
            let picked: Vec<usize> = (0..self.world.units.len())
                .filter(|&i| {
                    let u = &self.world.units[i];
                    u.alive && u.side == 0 && u.kind == Kind::Tank && u.pos.0 >= x0 && u.pos.0 <= x1 && u.pos.1 >= y0 && u.pos.1 <= y1
                })
                .collect();
            self.sel = picked;
            return;
        }
        let wp = self.to_world(p);
        if let Some(i) = self.own_at(wp) {
            self.sel = vec![i];
            return;
        }
        if self.touch && !self.sel.is_empty() {
            // タッチ：選んでいる部隊への命令
            let am = self.attack_mode;
            self.command(wp, am);
            self.attack_mode = false;
            return;
        }
        self.sel.clear();
    }

    fn key(&mut self, vk: u32, down: bool) {
        match vk {
            0x25 => self.keys[0] = down,
            0x26 => self.keys[1] = down,
            0x27 => self.keys[2] = down,
            0x28 => self.keys[3] = down,
            _ => {}
        }
        if !down {
            return;
        }
        match vk {
            0x1b => {
                if self.show_help {
                    self.show_help = false;
                } else if self.attack_mode {
                    self.attack_mode = false;
                } else {
                    self.sel.clear();
                }
            }
            0x70 | 0x48 => self.act(Act::Help),
            0x7a => self.act(Act::Fullscreen),
            _ => {}
        }
        if self.screen != Screen::Battle {
            if vk == 0x0d {
                self.act(Act::Start);
            }
            return;
        }
        match vk {
            0x20 => self.act(Act::Pause),
            0x41 => self.act(Act::AttackMode),
            0x53 => self.act(Act::Stop),
            0x44 => self.act(Act::Drone),
            0x51 => self.act(Act::AllTanks),
            0xbb | 0x6b => self.speed = (self.speed + 1).min(4),
            0xbd | 0x6d => self.speed = self.speed.saturating_sub(1).max(1),
            0x2e => {
                if self.world.phase == Phase::Prep {
                    self.tool = Tool::Remove;
                }
            }
            _ => {}
        }
    }

    fn own_at(&self, wp: (f32, f32)) -> Option<usize> {
        let r = TS * 0.65;
        (0..self.world.units.len())
            .filter(|&i| {
                let u = &self.world.units[i];
                u.alive && u.side == 0 && u.kind != Kind::Decoy && dist(u.pos, wp) < r
            })
            .min_by(|&a, &b| dist(self.world.units[a].pos, wp).partial_cmp(&dist(self.world.units[b].pos, wp)).unwrap_or(std::cmp::Ordering::Equal))
    }

    fn apply_tool(&mut self, wp: (f32, f32)) {
        let left = self.world.left(0);
        let ok = match self.tool {
            Tool::Tank(n, i) => {
                let ok = self.world.place_tank(0, n, i, wp);
                if !ok && n.tanks()[i].cost > left {
                    self.notify(t!("予算が足りない", "Not enough budget", "预算不足"));
                }
                ok
            }
            Tool::Decoy => {
                let (n, i) = self.mimic;
                self.world.place_decoy(0, n, i, wp)
            }
            Tool::Teeth => {
                self.painting = true;
                self.world.place_teeth(0, wp)
            }
            Tool::Remove => self.world.remove_at(0, wp),
        };
        if !ok && matches!(self.tool, Tool::Tank(..)) && self.world.left(0) >= self.tool_cost() {
            let t = tile_of(wp);
            if !in_zone(0, t.0, false) {
                self.notify(t!("戦車は自陣の配置範囲（明るい帯）にだけ置ける", "Tanks can only be deployed in your deployment zone (the bright strip)", "坦克只能部署在己方部署区（亮色区域）"));
            }
        }
        if !ok && matches!(self.tool, Tool::Decoy | Tool::Teeth) && self.world.left(0) < self.tool_cost() {
            self.notify(t!("予算が足りない", "Not enough budget", "预算不足"));
        }
    }

    fn tool_cost(&self) -> u32 {
        match self.tool {
            Tool::Tank(n, i) => n.tanks()[i].cost,
            Tool::Decoy => DECOY_COST,
            Tool::Teeth => TEETH_COST,
            Tool::Remove => 0,
        }
    }

    /// 選んだ部隊への命令（右クリック・タップ）
    fn command(&mut self, wp: (f32, f32), attack: bool) {
        if self.sel.is_empty() {
            return;
        }
        // 敵の戦車（デコイかもしれない）
        let enemy = (0..self.world.units.len()).filter(|&j| self.world.targetable_by(0, j) && dist(self.world.units[j].pos, wp) < TS * 0.75).min_by(|&a, &b| dist(self.world.units[a].pos, wp).partial_cmp(&dist(self.world.units[b].pos, wp)).unwrap_or(std::cmp::Ordering::Equal));
        let t = tile_of(wp);
        let teeth = {
            let k = self.world.teeth_at[idx(t.0, t.1)];
            (k != usize::MAX && self.world.teeth[k].alive && self.world.teeth[k].side == 1 && self.world.teeth[k].seen[0]).then_some(k)
        };
        let target = enemy.map(Target::Unit).or(teeth.map(Target::Teeth));
        let sel = self.sel.clone();
        let tanks: Vec<usize> = sel.iter().copied().filter(|&i| self.world.units[i].kind == Kind::Tank).collect();
        for &i in &sel {
            if self.world.units[i].kind == Kind::Drone {
                self.world.order(i, Order::Move { goal: wp, attack: false });
            }
        }
        if let Some(t) = target {
            for &i in &tanks {
                self.world.order(i, Order::Attack(t));
            }
            self.marker = Some((wp, 0.7, true));
            return;
        }
        // 隊形：目標の周りに並べる
        let n = tanks.len();
        let cols = (n as f32).sqrt().ceil() as usize;
        let mut sorted = tanks.clone();
        sorted.sort_by(|&a, &b| {
            let (pa, pb) = (self.world.units[a].pos, self.world.units[b].pos);
            (pa.1 + pa.0 * 0.01).partial_cmp(&(pb.1 + pb.0 * 0.01)).unwrap_or(std::cmp::Ordering::Equal)
        });
        for (k, &i) in sorted.iter().enumerate() {
            let (r, c) = (k / cols.max(1), k % cols.max(1));
            let off = ((c as f32 - (cols as f32 - 1.0) / 2.0) * 1.1 * TS, (r as f32 - ((n / cols.max(1)) as f32) / 2.0) * 1.1 * TS);
            let mut g = (wp.0 + off.1, wp.1 + off.0);
            let gt = tile_of(g);
            if !self.world.map.at(gt.0, gt.1).passable() {
                g = wp;
            }
            self.world.order(i, Order::Move { goal: g, attack });
        }
        self.marker = Some((wp, 0.7, attack));
    }

    fn act(&mut self, a: Act) {
        match a {
            Act::Field(i) => {
                self.field_i = i;
                self.regen_preview();
            }
            Act::Budget(i) => self.budget_i = i,
            Act::Diff(i) => self.diff_i = i,
            Act::Reroll => {
                self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223) | 1;
                self.regen_preview();
            }
            Act::Lang => {
                let l = lang::lang().next();
                lang::set_lang(l);
                crate::platform::save_lang(l);
                self.gfx.reload_fonts();
            }
            Act::Fullscreen => self.gfx.toggle_fullscreen(),
            Act::Start => {
                if self.screen == Screen::Title {
                    self.start_prep();
                }
            }
            Act::Tab(i) => {
                self.tab = i;
                if let Tool::Tank(..) = self.tool {
                    self.tool = Tool::Tank(NATIONS[i], 0);
                }
            }
            Act::Tool(t) => {
                if let Tool::Tank(n, i) = t {
                    self.mimic = (n, i);
                }
                self.tool = t;
            }
            Act::Auto => {
                self.world.clear_side(0);
                ai::prepare(&mut self.world, 0, self.seed ^ (self.clock * 1000.0) as u32);
            }
            Act::Clear => self.world.clear_side(0),
            Act::Begin => {
                if self.world.tank_count(0) == 0 {
                    self.notify(t!("戦車を1両以上配置してください", "Deploy at least one tank", "请至少部署一辆坦克"));
                    return;
                }
                self.world.start_battle();
                self.sel = (0..self.world.units.len()).filter(|&i| self.world.units[i].side == 0 && self.world.units[i].kind == Kind::Tank && self.world.units[i].alive).collect();
            }
            Act::Pause => self.paused = !self.paused,
            Act::Speed(s) => self.speed = s,
            Act::Help => self.show_help = !self.show_help,
            Act::Menu => {
                if self.menu_confirm > 0.0 || self.world.phase == Phase::Over {
                    self.screen = Screen::Title;
                    self.regen_preview();
                } else {
                    self.menu_confirm = 3.0;
                }
            }
            Act::AttackMode => {
                if !self.sel.is_empty() {
                    self.attack_mode = !self.attack_mode;
                }
            }
            Act::Stop => {
                for &i in &self.sel.clone() {
                    self.world.order(i, Order::Idle);
                }
            }
            Act::Drone => {
                if let Some(d) = self.world.drone(0) {
                    self.sel = vec![d];
                    self.cam = self.world.units[d].pos;
                    self.clamp_cam();
                }
            }
            Act::AllTanks => {
                self.sel = (0..self.world.units.len()).filter(|&i| self.world.units[i].side == 0 && self.world.units[i].kind == Kind::Tank && self.world.units[i].alive).collect();
            }
            Act::Again => {
                self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223) | 1;
                self.start_prep();
            }
            Act::ToTitle => {
                self.screen = Screen::Title;
                self.regen_preview();
            }
        }
    }

    // ------------------------------------------------------------ 描画

    fn render(&mut self) {
        self.gfx.begin([0.08, 0.09, 0.07, 1.0]);
        self.ensure_terrain();
        if self.screen == Screen::Title {
            // 地図を画面いっぱいに
            let z = (self.gfx.width as f32 / (MW as f32 * TS)).max(self.gfx.height as f32 / (MH as f32 * TS));
            self.zoom = z;
            self.cam = (MW as f32 * TS / 2.0, MH as f32 * TS / 2.0);
        }
        self.render_world();
        if self.screen == Screen::Battle {
            self.render_minimap();
        }
        if self.gfx.d2d_begin() {
            self.buttons.clear();
            self.panels.clear();
            if self.screen == Screen::Title {
                self.render_title();
            } else {
                self.render_labels();
                self.render_hud();
                if self.world.phase == Phase::Prep {
                    self.render_shop();
                } else {
                    self.render_selection();
                }
                if self.show_help {
                    self.render_help();
                }
                if self.world.phase == Phase::Over {
                    self.render_over();
                }
            }
            if let Some((s, t)) = &self.notice {
                let (s, a) = (s.clone(), t.min(1.0));
                let w = self.gfx.width as f32;
                let tw = self.gfx.measure(&s, Font::Big) + 40.0;
                let y = self.gfx.height as f32 * 0.24;
                self.gfx.fill_round(w / 2.0 - tw / 2.0, y, w / 2.0 + tw / 2.0, y + 40.0, 8.0, with_a(PANEL, a));
                self.gfx.text(&s, w / 2.0 - tw / 2.0, y, tw, 40.0, Font::BigCenter, with_a(GOLD, a));
            }
            self.gfx.d2d_end();
        }
        self.gfx.present();
    }

    fn set_world_xform(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let z = self.zoom;
        let sx = 2.0 * z / w;
        let sy = -2.0 * z / h;
        self.gfx.set_xform(sx, sy, -self.cam.0 * sx, -self.cam.1 * sy);
    }

    fn update_fog(&mut self) {
        let w = &self.world;
        for y in 0..MH {
            for x in 0..MW {
                let i = idx(x, y);
                let a = if self.screen == Screen::Title {
                    0.0
                } else {
                    match w.phase {
                        Phase::Prep => {
                            if x >= MW / 2 {
                                0.45
                            } else {
                                0.0
                            }
                        }
                        Phase::Battle => {
                            if w.vis[0][i] {
                                0.0
                            } else {
                                0.5
                            }
                        }
                        Phase::Over => 0.0,
                    }
                };
                self.fog_px[i] = (((a * 255.0) as u32) << 24) | 0x0a0c0a;
            }
        }
        if let Some(f) = &self.fog {
            self.gfx.update_texture(f, 0, 0, MW as u32, MH as u32, &self.fog_px);
        }
    }

    fn render_world(&mut self) {
        self.update_fog();
        self.set_world_xform();
        let (mw, mh) = (MW as f32 * TS, MH as f32 * TS);
        if let Some(t) = &self.terrain {
            self.gfx.draw_textured(t, 0.0, 0.0, mw, mh, [1.0; 4], true);
        }
        if self.screen == Screen::Title {
            self.gfx.rect(0.0, 0.0, mw, mh, [0.03, 0.04, 0.03, 0.45]);
            self.gfx.flush();
            return;
        }
        let mouse_w = self.to_world(self.mouse);
        let over = self.over_ui(self.mouse);
        let tool_cost = self.tool_cost();
        let left = self.world.left(0);
        let (tool, mimic, touch, screen, marker) = (self.tool, self.mimic, self.touch, self.screen, self.marker);
        let g = &mut self.gfx;
        let w = &self.world;
        let px = 1.0 / self.zoom;
        let clock = self.clock;
        // 準備：配置範囲
        if w.phase == Phase::Prep {
            let x1 = crate::map::DEPLOY as f32 * TS;
            g.rect(0.0, 0.0, x1, mh, [0.6, 0.8, 1.0, 0.10]);
            g.rect(x1 - 2.0 * px, 0.0, x1, mh, [0.6, 0.85, 1.0, 0.7]);
            let x2 = (MW / 2 - crate::map::HALF_MARGIN) as f32 * TS;
            g.rect(x1, 0.0, x2, mh, [1.0, 0.9, 0.5, 0.05]);
            g.rect(x2 - 2.0 * px, 0.0, x2, mh, [1.0, 0.85, 0.4, 0.55]);
        }
        // 基地
        for side in 0..2 {
            let b = w.base_pos(side);
            let col = side_color(side);
            g.circle(b.0, b.1, CAPTURE_R * TS, 48, with_a(col, 0.10));
            g.ring(b.0, b.1, CAPTURE_R * TS, 2.5 * px, 48, with_a(col, 0.8));
            let c = w.capture[side];
            if c > 0.0 {
                let seg = (48.0 * c).ceil() as usize;
                let oc = side_color(1 - side);
                let r = CAPTURE_R * TS;
                for k in 0..seg {
                    let a0 = -std::f32::consts::FRAC_PI_2 + k as f32 / 48.0 * std::f32::consts::TAU;
                    let a1 = a0 + std::f32::consts::TAU / 48.0;
                    let p0 = (b.0 + a0.cos() * r, b.1 + a0.sin() * r);
                    let p1 = (b.0 + a1.cos() * r, b.1 + a1.sin() * r);
                    g.line(p0, p1, 7.0 * px, oc);
                }
            }
            // 司令部の建物
            let s = TS * 0.75;
            g.rect(b.0 - s + 4.0, b.1 - s + 5.0, b.0 + s + 4.0, b.1 + s + 5.0, [0.0, 0.0, 0.0, 0.35]);
            g.rect(b.0 - s, b.1 - s, b.0 + s, b.1 + s, [0.42, 0.40, 0.36, 1.0]);
            g.rect(b.0 - s * 0.7, b.1 - s * 0.7, b.0 + s * 0.7, b.1 + s * 0.7, [0.34, 0.33, 0.30, 1.0]);
            g.rect_outline(b.0 - s, b.1 - s, b.0 + s, b.1 + s, 2.0, shade(col, 0.9));
            // 旗竿と旗
            g.line((b.0, b.1), (b.0, b.1 - TS * 1.3), 2.0, [0.2, 0.2, 0.2, 1.0]);
            let wave = (clock * 4.0).sin() * 2.0;
            g.tri((b.0, b.1 - TS * 1.3), (b.0 + TS * 0.8, b.1 - TS * 1.12 + wave), (b.0, b.1 - TS * 0.95), col);
        }
        // 竜の歯
        for tt in w.teeth.iter() {
            if !tt.alive || !(tt.side == 0 || tt.seen[0]) {
                continue;
            }
            let c = center(tt.tile);
            let dmg = tt.hp / crate::data::TEETH_HP;
            for (k, (ox, oy)) in [(-0.22, -0.22), (0.22, -0.18), (-0.18, 0.22), (0.24, 0.24), (0.0, 0.0)].iter().enumerate() {
                if k == 4 && dmg < 0.5 {
                    continue;
                }
                let (cx, cy) = (c.0 + ox * TS, c.1 + oy * TS);
                let r = TS * 0.17;
                g.tri((cx - r + 3.0, cy + r + 2.0), (cx + r + 3.0, cy + r + 2.0), (cx + 3.0, cy - r + 2.0), [0.0, 0.0, 0.0, 0.3]);
                g.tri((cx - r, cy + r), (cx, cy + r), (cx, cy - r), [0.78, 0.77, 0.72, 1.0]);
                g.tri((cx, cy + r), (cx + r, cy + r), (cx, cy - r), [0.55, 0.54, 0.50, 1.0]);
            }
            g.ring(c.0, c.1, TS * 0.48, 1.5 * px, 16, with_a(side_color(tt.side), 0.6));
        }
        // 部隊（残骸 → 戦車・デコイ）
        for pass in 0..2 {
            for (j, u) in w.units.iter().enumerate() {
                if u.kind == Kind::Drone {
                    continue;
                }
                if (pass == 0) == u.alive {
                    continue;
                }
                let visible = u.side == 0 || !u.alive || w.phase == Phase::Over || w.spotted[0].get(j).copied().unwrap_or(false);
                if !visible {
                    continue;
                }
                if !u.alive && u.kind == Kind::Decoy {
                    draw_decoy_wreck(g, u);
                    continue;
                }
                let alpha = if u.kind == Kind::Decoy && (u.side == 0 || u.revealed) { 0.55 } else { 1.0 };
                draw_tank(g, u, alpha, clock);
                if u.kind == Kind::Decoy && u.side == 1 && u.revealed && u.alive {
                    let r = TS * 0.35;
                    g.line((u.pos.0 - r, u.pos.1 - r), (u.pos.0 + r, u.pos.1 + r), 3.0 * px, [1.0, 0.9, 0.3, 0.9]);
                    g.line((u.pos.0 - r, u.pos.1 + r), (u.pos.0 + r, u.pos.1 - r), 3.0 * px, [1.0, 0.9, 0.3, 0.9]);
                }
            }
        }
        // 砲弾
        for s in &w.shells {
            let f = (s.t / s.dur).min(1.0);
            let p = (s.from.0 + (s.to.0 - s.from.0) * f, s.from.1 + (s.to.1 - s.from.1) * f);
            let tile = tile_of(p);
            if !w.vis[0][idx(tile.0, tile.1)] && s.side == 1 {
                continue;
            }
            let b = (f - 0.25).max(0.0);
            let q = (s.from.0 + (s.to.0 - s.from.0) * b, s.from.1 + (s.to.1 - s.from.1) * b);
            g.line(q, p, if s.big { 3.0 } else { 2.0 }, [1.0, 0.9, 0.5, 0.85]);
        }
        // 爆発・発砲炎
        for f in &w.fx {
            let k = f.t / f.life;
            let p = f.pos;
            match f.kind {
                FxKind::Flash => {
                    g.circle(p.0, p.1, TS * 0.35 * (1.0 - k), 10, [1.0, 0.85, 0.4, 0.9 * (1.0 - k)]);
                }
                FxKind::Spark => {
                    g.circle(p.0, p.1, TS * 0.3 * (0.5 + k), 10, [1.0, 0.6, 0.2, 0.9 * (1.0 - k)]);
                }
                FxKind::Dust => {
                    g.circle(p.0, p.1, TS * 0.45 * (0.4 + k), 12, [0.55, 0.48, 0.36, 0.55 * (1.0 - k)]);
                }
                FxKind::Boom => {
                    g.circle(p.0, p.1, TS * 1.1 * (0.3 + k), 18, [1.0, 0.55, 0.15, 0.85 * (1.0 - k)]);
                    g.circle(p.0, p.1, TS * 0.6 * (0.3 + k), 14, [1.0, 0.9, 0.5, 0.9 * (1.0 - k)]);
                }
            }
        }
        // 霧
        if let Some(f) = &self.fog {
            g.draw_textured(f, 0.0, 0.0, mw, mh, [1.0; 4], true);
        }
        // ドローン（空中なので霧の上）
        for (j, u) in w.units.iter().enumerate() {
            if u.kind != Kind::Drone || !u.alive {
                continue;
            }
            if u.side == 1 && !w.spotted[0].get(j).copied().unwrap_or(false) && w.phase != Phase::Over {
                continue;
            }
            draw_drone(g, u, clock);
            if u.side == 0 {
                g.ring(u.pos.0, u.pos.1, DRONE_SIGHT * TS, 1.5 * px, 64, [0.6, 0.85, 1.0, 0.25]);
            }
        }
        // 選択と体力
        for (j, u) in w.units.iter().enumerate() {
            if !u.alive || u.kind == Kind::Drone {
                continue;
            }
            let shown = u.side == 0 || w.spotted[0].get(j).copied().unwrap_or(false) || w.phase == Phase::Over;
            if !shown {
                continue;
            }
            let s = u.spec();
            let selected = self.sel.contains(&j);
            if selected {
                g.ring(u.pos.0, u.pos.1, s.len * TS * 0.62, 2.0 * px, 24, [0.5, 1.0, 0.5, 0.95]);
                if let Order::Move { goal, attack } = u.order {
                    g.line(u.pos, goal, 1.2 * px, if attack { [1.0, 0.5, 0.3, 0.45] } else { [0.5, 1.0, 0.5, 0.45] });
                }
                if let Order::Attack(t) = u.order {
                    g.line(u.pos, w.target_pos(t), 1.2 * px, [1.0, 0.3, 0.2, 0.6]);
                }
            }
            if w.phase == Phase::Battle && u.kind == Kind::Tank || (u.kind == Kind::Decoy && u.side == 1 && w.phase == Phase::Battle) {
                // デコイも本物らしく見せる
                let frac = if u.kind == Kind::Decoy { 1.0 } else { u.hp / s.hp };
                let (bw, by) = (s.len * TS * 0.9, u.pos.1 - s.len * TS * 0.7);
                g.rect(u.pos.0 - bw / 2.0, by, u.pos.0 + bw / 2.0, by + 4.0 * px.max(0.6), [0.0, 0.0, 0.0, 0.6]);
                g.rect(u.pos.0 - bw / 2.0, by, u.pos.0 - bw / 2.0 + bw * frac, by + 4.0 * px.max(0.6), side_color(u.side));
            }
        }
        // 準備：置く物の予告
        if w.phase == Phase::Prep && screen == Screen::Battle && !over && !touch {
            let wp = mouse_w;
            let t = tile_of(wp);
            let c = center(t);
            let (ok, r) = match tool {
                Tool::Tank(..) => (w.can_place(0, wp, false, false) && left >= tool_cost, TS * 0.45),
                Tool::Decoy => (w.can_place(0, wp, true, false) && left >= DECOY_COST, TS * 0.45),
                Tool::Teeth => (w.can_place(0, wp, true, true) && left >= TEETH_COST, TS * 0.5),
                Tool::Remove => (true, TS * 0.5),
            };
            let col = if tool == Tool::Remove { [1.0, 0.6, 0.3, 0.8] } else if ok { [0.5, 1.0, 0.5, 0.8] } else { [1.0, 0.3, 0.3, 0.8] };
            g.rect_outline(c.0 - r, c.1 - r, c.0 + r, c.1 + r, 2.0 * px, col);
            if let (Tool::Tank(n, i), true) = (tool, ok) {
                let ghost = ghost_unit(n, i, c);
                draw_tank(g, &ghost, 0.5, clock);
                g.ring(c.0, c.1, n.tanks()[i].range * TS, 1.5 * px, 64, [1.0, 1.0, 1.0, 0.2]);
            }
            if let (Tool::Decoy, true) = (tool, ok) {
                let ghost = ghost_unit(mimic.0, mimic.1, c);
                draw_tank(g, &ghost, 0.4, clock);
                g.ring(c.0, c.1, DECOY_REVEAL * TS, 1.5 * px, 48, [1.0, 0.9, 0.4, 0.3]);
            }
        }
        // 命令の印
        if let Some((p, t, attack)) = marker {
            let k = t / 0.7;
            let col = if attack { [1.0, 0.45, 0.3, k] } else { [0.5, 1.0, 0.5, k] };
            g.ring(p.0, p.1, TS * (0.7 - 0.4 * k), 2.5 * px, 20, col);
        }
        g.flush();
    }

    fn render_minimap(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let mw = if self.gfx.compact { 150.0 } else { 220.0 };
        let mh = mw * MH as f32 / MW as f32;
        let (x0, y1) = (8.0, h - 8.0);
        let y0 = y1 - mh;
        let x1 = x0 + mw;
        self.minimap = [x0, y0, x1, y1];
        self.gfx.set_screen_xform();
        let view_a = self.to_world((0.0, 0.0));
        let view_b = self.to_world((w, h));
        let g = &mut self.gfx;
        g.rect(x0 - 3.0, y0 - 3.0, x1 + 3.0, y1 + 3.0, [0.05, 0.05, 0.04, 0.95]);
        g.rect_outline(x0 - 3.0, y0 - 3.0, x1 + 3.0, y1 + 3.0, 1.5, EDGE);
        if let Some(t) = &self.terrain {
            g.draw_textured(t, x0, y0, x1, y1, [1.0; 4], true);
        }
        if let Some(f) = &self.fog {
            g.draw_textured(f, x0, y0, x1, y1, [1.0; 4], true);
        }
        let sx = mw / (MW as f32 * TS);
        let sy = mh / (MH as f32 * TS);
        let m = |p: (f32, f32)| (x0 + p.0 * sx, y0 + p.1 * sy);
        let wd = &self.world;
        for side in 0..2 {
            let b = m(wd.base_pos(side));
            g.rect(b.0 - 4.0, b.1 - 4.0, b.0 + 4.0, b.1 + 4.0, side_color(side));
            g.rect_outline(b.0 - 4.0, b.1 - 4.0, b.0 + 4.0, b.1 + 4.0, 1.0, [0.0, 0.0, 0.0, 1.0]);
        }
        for tt in &wd.teeth {
            if tt.alive && (tt.side == 0 || tt.seen[0]) {
                let p = m(center(tt.tile));
                g.rect(p.0 - 1.0, p.1 - 1.0, p.0 + 1.0, p.1 + 1.0, [0.85, 0.85, 0.8, 1.0]);
            }
        }
        for (j, u) in wd.units.iter().enumerate() {
            if !u.alive {
                continue;
            }
            let shown = u.side == 0 || wd.spotted[0].get(j).copied().unwrap_or(false);
            if !shown || (u.kind == Kind::Decoy && u.side == 1 && u.revealed) {
                continue;
            }
            let p = m(u.pos);
            let r = if u.kind == Kind::Drone { 2.0 } else { 2.5 };
            let mut c = side_color(u.side);
            if u.kind == Kind::Decoy && u.side == 0 {
                c = with_a(c, 0.45);
            }
            if self.sel.contains(&j) {
                c = [0.6, 1.0, 0.6, 1.0];
            }
            if u.kind == Kind::Drone {
                g.tri((p.0, p.1 - r - 1.0), (p.0 + r + 1.0, p.1 + r), (p.0 - r - 1.0, p.1 + r), c);
            } else {
                g.rect(p.0 - r, p.1 - r, p.0 + r, p.1 + r, c);
            }
        }
        for &(p, age) in &self.pings {
            let q = m(p);
            let k = (age / 2.5).min(1.0);
            g.ring(q.0, q.1, 4.0 + 14.0 * k, 1.5, 16, [1.0, 0.85, 0.3, 1.0 - k]);
        }
        // 見えている範囲
        let (a, b) = (m(view_a), m(view_b));
        g.rect_outline(a.0.max(x0), a.1.max(y0), b.0.min(x1), b.1.min(y1), 1.2, [1.0, 1.0, 1.0, 0.8]);
        g.flush();
    }

    // ------------------------------------------------------------ 画面部品

    fn hovered(&self, r: [f32; 4]) -> bool {
        !self.touch && self.mouse.0 >= r[0] && self.mouse.0 <= r[2] && self.mouse.1 >= r[1] && self.mouse.1 <= r[3]
    }

    fn button(&mut self, r: [f32; 4], label: &str, act: Act, active: bool) {
        let hov = self.hovered(r);
        let bg = if active {
            [0.55, 0.42, 0.16, 0.95]
        } else if hov {
            [0.26, 0.26, 0.20, 0.95]
        } else {
            [0.15, 0.16, 0.12, 0.92]
        };
        self.gfx.fill_round(r[0], r[1], r[2], r[3], 5.0, bg);
        self.gfx.stroke_round(r[0], r[1], r[2], r[3], 5.0, 1.0, if active { GOLD } else { EDGE });
        self.gfx.text(label, r[0] + 3.0, r[1], r[2] - r[0] - 6.0, r[3] - r[1], Font::NormalCenter, if active { [1.0, 0.97, 0.88, 1.0] } else { TEXT });
        self.buttons.push(Btn { r, act });
    }

    fn panel(&mut self, r: [f32; 4]) {
        self.gfx.fill_round(r[0], r[1], r[2], r[3], 8.0, PANEL);
        self.gfx.stroke_round(r[0], r[1], r[2], r[3], 8.0, 1.0, EDGE);
        self.panels.push(r);
    }

    fn render_title(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let compact = self.gfx.compact;
        // 題字
        let ty = if compact { 6.0 } else { h * 0.06 };
        self.gfx.text_shadow(t!("鋼鉄の大地", "Steel Front", "钢铁大地"), 0.0, ty, w, 50.0, Font::Title, GOLD);
        self.gfx.text_shadow(t!("第二次世界大戦 戦車戦", "WWII tank battles", "二战坦克战"), 0.0, ty + 46.0, w, 26.0, Font::BigCenter, TEXT);
        let pw = (w - 40.0).min(980.0);
        let x0 = (w - pw) / 2.0;
        let y0 = ty + if compact { 76.0 } else { 90.0 };
        let y1 = (h - 12.0).min(y0 + 560.0);
        self.panel([x0, y0, x0 + pw, y1]);
        let col_w = pw * 0.48;
        // 左列：戦場
        let lx = x0 + 18.0;
        let mut y = y0 + 12.0;
        self.gfx.text(t!("戦場", "Battlefield", "战场"), lx, y, col_w, 24.0, Font::Big, GOLD);
        y += 28.0;
        let bh = if compact { 30.0 } else { 36.0 };
        for (i, f) in FIELDS.iter().enumerate() {
            let r = [lx, y, lx + col_w, y + bh - 4.0];
            self.button(r, f.name(), Act::Field(i), i == self.field_i);
            y += bh;
        }
        y += 4.0;
        let desc = self.field().desc();
        self.gfx.text(desc, lx, y, col_w, 60.0, Font::Normal, TEXT);
        y += self.gfx.measure_h(desc, Font::Normal, col_w) + 8.0;
        self.button([lx, y, lx + col_w * 0.6, y + 30.0], t!("地形を作り直す", "Regenerate terrain", "重新生成地形"), Act::Reroll, false);
        // 右列：予算・難易度
        let rx = x0 + pw * 0.52;
        let rw = pw * 0.48 - 18.0;
        let mut y = y0 + 12.0;
        self.gfx.text(t!("予算規模（双方同じ）", "Budget (same for both sides)", "预算规模（双方相同）"), rx, y, rw, 24.0, Font::Big, GOLD);
        y += 28.0;
        let bw = (rw - 16.0) / 3.0;
        for (i, b) in BUDGETS.iter().enumerate() {
            let r = [rx + i as f32 * (bw + 8.0), y, rx + i as f32 * (bw + 8.0) + bw, y + 34.0];
            self.button(r, &format!("{}  {}", b.name(), b.points()), Act::Budget(i), i == self.budget_i);
        }
        y += 44.0;
        self.gfx.text(t!("敵の強さ", "Enemy strength", "敌方强度"), rx, y, rw, 24.0, Font::Big, GOLD);
        y += 28.0;
        for i in 0..3 {
            let r = [rx + i as f32 * (bw + 8.0), y, rx + i as f32 * (bw + 8.0) + bw, y + 34.0];
            self.button(r, diff_name(i), Act::Diff(i), i == self.diff_i);
        }
        y += 46.0;
        let rules = t!(
            "・どの国の戦車も自由に組み合わせて買える。ドイツ・ソ連の強力な戦車は高く、日本・イタリアの戦車は安い\n・開戦前にデコイ（偽の戦車）と竜の歯（対戦車障害物）を置ける。開戦後は追加できない\n・偵察ドローンを1機ずつ使える。戦力には数えず、攻撃もされない\n・敵の基地を占領するか、敵の戦車をすべて撃破すれば勝ち。森・湿地・丘では進撃が遅くなる",
            "- Mix tanks from any nation. Powerful German and Soviet tanks are expensive; Japanese and Italian tanks are cheap.\n- Before battle, place decoys (fake tanks) and dragon's teeth (anti-tank obstacles). None can be added once battle begins.\n- Each side has one recon drone. It is not a combat unit and cannot be attacked.\n- Win by capturing the enemy base or destroying all enemy tanks. Forest, marsh and hills slow your advance.",
            "・可自由组合任意国家的坦克。德、苏强力坦克昂贵，日、意坦克便宜\n・开战前可布置诱饵（假坦克）和龙牙（反坦克障碍），开战后不能追加\n・双方各有一架侦察无人机，不计入战力，也不会被攻击\n・占领敌方基地或击毁敌方全部坦克即获胜。森林、沼泽、丘陵会减慢推进"
        );
        let rh = (y1 - 60.0 - y).max(40.0);
        self.gfx.text(rules, rx, y, rw, rh, Font::Small, DIM);
        let by = y1 - 52.0;
        self.button([rx, by, rx + rw * 0.62, by + 40.0], t!("準備へ進む ▶", "To deployment ▶", "进入准备 ▶"), Act::Start, true);
        let lw = rw * 0.36;
        self.button([rx + rw - lw, by, rx + rw, by + 18.0], lang::lang().next().native(), Act::Lang, false);
        self.button([rx + rw - lw, by + 22.0, rx + rw, by + 40.0], t!("全画面", "Fullscreen", "全屏"), Act::Fullscreen, false);
    }

    fn render_hud(&mut self) {
        let w = self.gfx.width as f32;
        let bar = 36.0;
        self.panel([0.0, 0.0, w, bar]);
        let wd = &self.world;
        let field = wd.map.field.name().to_string();
        let status = match wd.phase {
            Phase::Prep => tf!("準備　予算 {} / {}", "Deployment — budget {} / {}", "准备　预算 {} / {}", wd.left(0), wd.budget),
            Phase::Battle | Phase::Over => {
                let t = wd.time as u32;
                let lim = TIME_LIMIT as u32;
                tf!("戦闘 {:02}:{:02} / {:02}:00", "Battle {:02}:{:02} / {:02}:00", "战斗 {:02}:{:02} / {:02}:00", t / 60, t % 60, lim / 60)
            }
        };
        let forces = {
            let mine = wd.tank_count(0);
            let lost = wd.units.iter().filter(|u| u.side == 0 && u.kind == Kind::Tank && !u.alive).count();
            let kills = wd.units.iter().filter(|u| u.side == 1 && u.kind == Kind::Tank && !u.alive).count();
            tf!("自軍 {}両（損失 {}）　撃破 {}", "Ours: {} tanks (lost {})   Kills: {}", "我方 {}辆（损失 {}）　击毁 {}", mine, lost, kills)
        };
        let compact = self.gfx.compact;
        let fw = self.gfx.measure(&field, Font::Big).min(300.0);
        self.gfx.text(&field, 10.0, 0.0, fw, bar, Font::Big, GOLD);
        let sx = 10.0 + fw + 24.0;
        let sw = self.gfx.measure(&status, Font::Normal);
        self.gfx.text(&status, sx, 0.0, sw + 2.0, bar, Font::Normal, TEXT);
        if !compact && self.world.phase != Phase::Prep {
            let fx = sx + sw + 24.0;
            let room = (w - 480.0 - fx).max(0.0);
            if room > 120.0 {
                self.gfx.text(&forces, fx, 0.0, room, bar, Font::Normal, TEXT);
            }
        }
        // 右側のボタン
        let mut x = w - 8.0;
        let menu_label = if self.menu_confirm > 0.0 { t!("もう一度押すと終了", "Click again to quit", "再按一次退出") } else { t!("タイトルへ", "Title", "返回标题") };
        let mw = if self.menu_confirm > 0.0 { 150.0 } else { 86.0 };
        self.button([x - mw, 5.0, x, bar - 5.0], menu_label, Act::Menu, self.menu_confirm > 0.0);
        x -= mw + 6.0;
        self.button([x - 34.0, 5.0, x, bar - 5.0], "?", Act::Help, self.show_help);
        x -= 40.0;
        if self.world.phase == Phase::Battle {
            for s in (1..=3).rev() {
                self.button([x - 38.0, 5.0, x, bar - 5.0], &format!("x{}", s), Act::Speed(s), self.speed == s);
                x -= 42.0;
            }
            let pl = if self.paused { t!("再開", "Resume", "继续") } else { t!("一時停止", "Pause", "暂停") };
            self.button([x - 80.0, 5.0, x, bar - 5.0], pl, Act::Pause, self.paused);
            // 占領の度合い
            let wd = &self.world;
            let caps = [wd.capture[0], wd.capture[1]];
            let y = bar + 6.0;
            let bx = w / 2.0 - 160.0;
            if caps[0] > 0.0 || caps[1] > 0.0 {
                self.gfx.fill_round(bx - 6.0, y - 2.0, bx + 326.0, y + 48.0, 6.0, PANEL);
                for (k, side) in [0usize, 1].into_iter().enumerate() {
                    let yy = y + k as f32 * 24.0;
                    let label = if side == 0 { t!("自軍基地", "Our base", "我方基地") } else { t!("敵基地", "Enemy base", "敌方基地") };
                    self.gfx.text(label, bx, yy, 90.0, 20.0, Font::Small, TEXT);
                    self.gfx.fill_rect(bx + 90.0, yy + 5.0, bx + 320.0, yy + 15.0, [0.0, 0.0, 0.0, 0.6]);
                    self.gfx.fill_rect(bx + 90.0, yy + 5.0, bx + 90.0 + 230.0 * caps[side], yy + 15.0, side_color(1 - side));
                }
            }
        }
        // 出来事の記録
        let lw = if compact { 300.0 } else { 380.0 };
        let mut y = bar + 8.0;
        let lx = if self.world.phase == Phase::Prep { 10.0 } else { w - lw - 10.0 };
        let lines: Vec<(String, f32, bool)> = self.log.iter().rev().take(6).filter(|l| l.age < 10.0).map(|l| (l.text.clone(), l.age, l.good)).collect();
        for (text, age, good) in lines.into_iter() {
            let a = (10.0 - age).min(1.0);
            let th = self.gfx.measure_h(&text, Font::Small, lw - 12.0).max(16.0) + 6.0;
            self.gfx.fill_round(lx, y, lx + lw, y + th, 4.0, [0.0, 0.0, 0.0, 0.5 * a]);
            self.gfx.text(&text, lx + 6.0, y + 3.0, lw - 12.0, th - 6.0, Font::Small, with_a(if good { GOOD } else { [1.0, 0.7, 0.6, 1.0] }, a));
            y += th + 3.0;
        }
        if self.attack_mode {
            let (mx, my) = self.mouse;
            self.gfx.text_shadow(t!("攻撃移動：目標をクリック", "Attack-move: click a target", "攻击移动：点击目标"), mx + 16.0, my + 10.0, 240.0, 20.0, Font::Small, [1.0, 0.6, 0.4, 1.0]);
        }
        // 範囲選択の枠
        if let Some(d) = self.ldown {
            if !self.in_minimap(d) {
                let m = self.mouse;
                if (m.0 - d.0).abs() > 8.0 || (m.1 - d.1).abs() > 8.0 {
                    self.gfx.fill_rect(d.0.min(m.0), d.1.min(m.1), d.0.max(m.0), d.1.max(m.1), [0.5, 1.0, 0.5, 0.12]);
                    self.gfx.stroke_rect(d.0.min(m.0), d.1.min(m.1), d.0.max(m.0), d.1.max(m.1), 1.0, [0.5, 1.0, 0.5, 0.9]);
                }
            }
        }
        // ミニマップの見出し
        let r = self.minimap;
        self.panels.push([r[0] - 3.0, r[1] - 3.0, r[2] + 3.0, r[3] + 3.0]);
    }

    /// 部隊の名前など、地図の上の文字（重ならないよう、指している部隊と1両だけ選んだ部隊に）
    fn render_labels(&mut self) {
        let mw = self.to_world(self.mouse);
        let mut items: Vec<(String, (f32, f32), C)> = Vec::new();
        for (j, u) in self.world.units.iter().enumerate() {
            if !u.alive || u.kind == Kind::Drone {
                continue;
            }
            let shown = u.side == 0 || self.world.spotted[0].get(j).copied().unwrap_or(false);
            if !shown {
                continue;
            }
            let hover = !self.touch && dist(u.pos, mw) < TS * 0.6;
            let decoy_tag = u.kind == Kind::Decoy && (u.side == 0 || u.revealed);
            if decoy_tag && self.zoom > 0.7 {
                items.push((t!("デコイ", "Decoy", "诱饵").to_string(), u.pos, [1.0, 0.9, 0.5, 0.9]));
            } else if hover || (self.sel.len() == 1 && self.sel[0] == j) {
                let name = if u.side == 1 { tf!("敵 {}", "Enemy {}", "敌 {}", u.spec().name()) } else { u.spec().name().to_string() };
                items.push((name, u.pos, with_a(side_color(u.side), 0.95)));
            }
        }
        for (s, p, c) in items {
            let sp = self.to_screen((p.0, p.1 + TS * 0.55));
            self.gfx.text_shadow(&s, sp.0 - 90.0, sp.1, 180.0, 18.0, Font::NormalCenter, c);
        }
    }

    fn render_shop(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let compact = self.gfx.compact;
        let pw = if compact { 300.0 } else { 340.0 };
        let x0 = w - pw - 8.0;
        let y0 = 44.0;
        let y1 = h - 8.0;
        self.panel([x0, y0, x0 + pw, y1]);
        let ix = x0 + 10.0;
        let iw = pw - 20.0;
        let mut y = y0 + 8.0;
        // 国の見出し
        let tw = (iw - 10.0) / 3.0;
        for (k, n) in NATIONS.iter().enumerate() {
            let (r, c) = (k / 3, k % 3);
            let rr = [ix + c as f32 * (tw + 5.0), y + r as f32 * 30.0, ix + c as f32 * (tw + 5.0) + tw, y + r as f32 * 30.0 + 26.0];
            self.button(rr, n.name(), Act::Tab(k), self.tab == k);
            // 旗の色
            let f = n.flag();
            for (s, col) in f.iter().enumerate() {
                self.gfx.fill_rect(rr[0] + 4.0 + s as f32 * 4.0, rr[3] - 5.0, rr[0] + 8.0 + s as f32 * 4.0, rr[3] - 2.0, *col);
            }
        }
        y += 62.0;
        let nation = NATIONS[self.tab];
        if !compact {
            self.gfx.text(nation.blurb(), ix, y, iw, 34.0, Font::Small, DIM);
            y += 34.0;
        }
        let ch = if compact { 48.0 } else { 62.0 };
        for (i, s) in nation.tanks().iter().enumerate() {
            let r = [ix, y, ix + iw, y + ch - 4.0];
            let active = self.tool == Tool::Tank(nation, i);
            let hov = self.hovered(r);
            let afford = s.cost <= self.world.left(0);
            self.gfx.fill_round(r[0], r[1], r[2], r[3], 5.0, if active { [0.45, 0.35, 0.14, 0.95] } else if hov { [0.24, 0.24, 0.19, 0.95] } else { [0.14, 0.15, 0.11, 0.92] });
            self.gfx.stroke_round(r[0], r[1], r[2], r[3], 5.0, 1.0, if active { GOLD } else { EDGE });
            let nc = if afford { TEXT } else { DIM };
            self.gfx.text(s.name(), r[0] + 8.0, r[1] + 2.0, iw - 80.0, 20.0, Font::Normal, nc);
            self.gfx.text(&format!("{}", s.cost), r[2] - 70.0, r[1] + 2.0, 62.0, 20.0, Font::Big, if afford { GOLD } else { RED });
            let stats = tf!(
                "{} 装甲{}/{} 貫通{} 速{:.1} 射程{}",
                "{} Armor {}/{} Pen {} Spd {:.1} Rng {}",
                "{} 装甲{}/{} 穿深{} 速{:.1} 射程{}",
                weight_name(s.weight),
                s.front as u32,
                s.side as u32,
                s.pen as u32,
                s.speed,
                s.range as u32
            );
            self.gfx.text(&stats, r[0] + 8.0, r[1] + 22.0, iw - 16.0, 18.0, Font::Small, DIM);
            if !compact {
                self.gfx.text(&format!("{}  HP {}", s.gun, s.hp as u32), r[0] + 8.0, r[1] + 39.0, iw - 16.0, 18.0, Font::Small, DIM);
            }
            self.buttons.push(Btn { r, act: Act::Tool(Tool::Tank(nation, i)) });
            y += ch;
        }
        y += 4.0;
        let (mn, mi) = self.mimic;
        let decoy_label = tf!("デコイ {}（{}に偽装）", "Decoy {} (looks like {})", "诱饵 {}（伪装成{}）", DECOY_COST, mn.tanks()[mi].name());
        self.button([ix, y, ix + iw, y + 28.0], &decoy_label, Act::Tool(Tool::Decoy), self.tool == Tool::Decoy);
        y += 32.0;
        let hw = (iw - 6.0) / 2.0;
        self.button([ix, y, ix + hw, y + 28.0], &tf!("竜の歯 {}", "Dragon's teeth {}", "龙牙 {}", TEETH_COST), Act::Tool(Tool::Teeth), self.tool == Tool::Teeth);
        self.button([ix + hw + 6.0, y, ix + iw, y + 28.0], t!("撤去", "Remove", "撤除"), Act::Tool(Tool::Remove), self.tool == Tool::Remove);
        y += 34.0;
        let wd = &self.world;
        let tanks = wd.tank_count(0);
        let decoys = wd.units.iter().filter(|u| u.side == 0 && u.kind == Kind::Decoy).count();
        let teeth = wd.teeth.iter().filter(|t| t.side == 0).count();
        let summary = tf!("戦車 {}両　デコイ {}　竜の歯 {}", "Tanks {}  Decoys {}  Teeth {}", "坦克 {}辆　诱饵 {}　龙牙 {}", tanks, decoys, teeth);
        self.gfx.text(&summary, ix, y, iw, 20.0, Font::Normal, TEXT);
        y += 22.0;
        if !compact {
            let hint = t!("左クリックで置く・右クリックで取り除く。竜の歯はドラッグで連続して置ける", "Left-click to place, right-click to remove. Drag to lay a line of teeth.", "左键放置，右键撤除。拖动可连续放置龙牙");
            self.gfx.text(hint, ix, y, iw, 36.0, Font::Small, DIM);
        }
        let by = y1 - 48.0;
        self.button([ix, by - 34.0, ix + hw, by - 6.0], t!("おまかせ配置", "Auto deploy", "自动部署"), Act::Auto, false);
        self.button([ix + hw + 6.0, by - 34.0, ix + iw, by - 6.0], t!("全て撤去", "Clear all", "全部撤除"), Act::Clear, false);
        self.button([ix, by, ix + iw, by + 40.0], t!("戦闘開始 ▶", "Begin battle ▶", "开始战斗 ▶"), Act::Begin, true);
    }

    fn render_selection(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let compact = self.gfx.compact;
        // 命令ボタン
        let bx = self.minimap[2] + 14.0;
        let bw = if compact { 74.0 } else { 96.0 };
        let by = h - 40.0;
        let acts = [
            (t!("攻撃移動 A", "Attack A", "攻击移动 A"), Act::AttackMode, self.attack_mode),
            (t!("停止 S", "Stop S", "停止 S"), Act::Stop, false),
            (t!("全戦車 Q", "All tanks Q", "全部坦克 Q"), Act::AllTanks, false),
            (t!("ドローン D", "Drone D", "无人机 D"), Act::Drone, false),
        ];
        for (k, (label, a, on)) in acts.into_iter().enumerate() {
            let x = bx + k as f32 * (bw + 6.0);
            self.button([x, by, x + bw, by + 32.0], label, a, on);
        }
        if self.sel.is_empty() {
            return;
        }
        let pw = if compact { 360.0 } else { 440.0 };
        let x0 = bx;
        let y1 = by - 6.0;
        let y0 = y1 - if compact { 66.0 } else { 82.0 };
        if x0 + pw > w {
            return;
        }
        self.panel([x0, y0, x0 + pw, y1]);
        let wd = &self.world;
        if self.sel.len() == 1 {
            let u = &wd.units[self.sel[0]];
            if u.kind == Kind::Drone {
                self.gfx.text(t!("偵察ドローン", "Recon drone", "侦察无人机"), x0 + 10.0, y0 + 4.0, pw - 20.0, 22.0, Font::Big, GOLD);
                let d = tf!("視界 {}マス・地形を無視して飛ぶ。戦力には数えず、攻撃されない。右クリックで移動", "Sight {} tiles, flies over any terrain. Not a combat unit and never attacked. Right-click to move.", "视野 {}格，无视地形飞行。不计入战力，也不会被攻击。右键移动", DRONE_SIGHT as u32);
                self.gfx.text(&d, x0 + 10.0, y0 + 28.0, pw - 20.0, 48.0, Font::Small, TEXT);
                return;
            }
            let s = u.spec();
            let terr = wd.map.at(u.tile().0, u.tile().1);
            self.gfx.text(&format!("{}（{}）", s.name(), u.nation.name()), x0 + 10.0, y0 + 4.0, pw - 20.0, 22.0, Font::Big, GOLD);
            let l1 = tf!("耐久 {}/{}　装甲 正面{} 側面{} 背面{}　貫通 {}", "HP {}/{}  Armor F{} S{} R{}  Pen {}", "耐久 {}/{}　装甲 正{} 侧{} 后{}　穿深 {}", u.hp.ceil() as u32, s.hp as u32, s.front as u32, s.side as u32, s.rear() as u32, s.pen as u32);
            let l2 = tf!("地形：{}（速度 x{:.2}）　撃破 {}", "Terrain: {} (speed x{:.2})  Kills {}", "地形：{}（速度 x{:.2}）　击毁 {}", terr.name(), terr.speed(s.weight), u.kills);
            self.gfx.text(&l1, x0 + 10.0, y0 + 28.0, pw - 20.0, 20.0, Font::Small, TEXT);
            self.gfx.text(&l2, x0 + 10.0, y0 + 48.0, pw - 20.0, 20.0, Font::Small, TEXT);
        } else {
            let n = self.sel.len();
            let hp: f32 = self.sel.iter().map(|&i| wd.units[i].hp / wd.units[i].spec().hp.max(1.0)).sum::<f32>() / n as f32;
            self.gfx.text(&tf!("{}両を選択中", "{} selected", "已选择 {}辆", n), x0 + 10.0, y0 + 4.0, pw - 20.0, 22.0, Font::Big, GOLD);
            let mut names: Vec<&str> = self.sel.iter().map(|&i| wd.units[i].spec().name()).collect();
            names.sort();
            names.dedup();
            let l = tf!("平均耐久 {}%　{}", "Avg HP {}%  {}", "平均耐久 {}%　{}", (hp * 100.0) as u32, names.join(", "));
            self.gfx.text(&l, x0 + 10.0, y0 + 28.0, pw - 20.0, 44.0, Font::Small, TEXT);
        }
    }

    fn render_help(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pw = (w - 40.0).min(760.0);
        let ph = (h - 80.0).min(520.0);
        let (x0, y0) = ((w - pw) / 2.0, (h - ph) / 2.0);
        self.panel([x0, y0, x0 + pw, y0 + ph]);
        self.gfx.text(t!("操作と規則", "Controls & rules", "操作与规则"), x0, y0 + 8.0, pw, 30.0, Font::BigCenter, GOLD);
        let body = t!(
            "【準備】右の一覧から戦車を選び、自陣の明るい帯に左クリックで配置。デコイと竜の歯は中央の手前まで置ける。右クリックで取り除き、予算が戻る。\n【戦闘】左クリック／ドラッグで自軍を選択、右クリックで移動。敵を右クリックすると攻撃。A キーのあと地点を指定すると、敵を見つけ次第止まって撃つ攻撃移動。S 停止、Q 全戦車、D ドローン、Space 一時停止、＋／－ 速度。\n【地図】矢印キー・画面端・中ボタンのドラッグで移動、ホイールで拡大縮小、ミニマップで移動。\n【装甲】正面・側面・背面で装甲が違う。側面や背面を狙えば軽戦車でも重戦車を抜ける。移動中は命中率が下がる。\n【視界】森・村落・生垣の中の戦車は近づかないと見つからないが、撃つと位置がばれる。丘の上は遠くまで見える。\n【デコイ】敵には本物の戦車に見え、攻撃を引きつける。敵の戦車が近づくと見破られる。\n【竜の歯】戦車は通れない。敵は砲撃して壊さなければならない。\n【勝利】敵基地の円の中に自軍の戦車だけがいると占領が進む。占領するか全滅させれば勝ち。15分で決着しなければ残存戦力で判定。",
            "[Deploy] Pick a tank on the right and left-click inside your bright zone. Decoys and teeth may go up to just short of the centre. Right-click removes an item and refunds it.\n[Battle] Left-click/drag to select, right-click to move. Right-click an enemy to attack. Press A then pick a point to attack-move (halt and fire at enemies). S stop, Q all tanks, D drone, Space pause, +/- speed.\n[Map] Arrow keys, screen edges or middle-drag to scroll; wheel to zoom; click the minimap.\n[Armor] Front, side and rear armor differ. Hit the side or rear and even light tanks can kill heavies. Firing on the move is less accurate.\n[Sight] Tanks in forest, villages or hedgerows are only spotted up close — until they fire. Hills see further.\n[Decoys] Look like real tanks to the enemy and draw fire, until an enemy tank comes close.\n[Dragon's teeth] Block tanks. The enemy must shoot them apart.\n[Victory] Your tanks alone inside the enemy base circle capture it. Capture the base or destroy every enemy tank. After 15 minutes the remaining strength decides.",
            "【准备】从右侧列表选择坦克，在己方亮色区域左键部署。诱饵和龙牙可放置到中线前。右键撤除并退还预算。\n【战斗】左键/拖动选择部队，右键移动，右键点击敌人攻击。按A后指定地点为攻击移动（发现敌人即停下射击）。S停止，Q全部坦克，D无人机，空格暂停，＋/－速度。\n【地图】方向键、屏幕边缘或中键拖动移动，滚轮缩放，点击小地图移动。\n【装甲】正面、侧面、后部装甲不同。打击侧后方，轻坦也能击穿重坦。行进间射击命中率下降。\n【视野】森林、村落、树篱中的坦克只有接近才能发现，但开火会暴露。丘陵上视野更远。\n【诱饵】在敌人看来是真坦克，吸引火力，敌方坦克接近时会被识破。\n【龙牙】坦克无法通过，敌人必须炮击摧毁。\n【胜利】敌方基地圈内只有我方坦克时开始占领。占领或全歼即获胜。15分钟未分胜负则按剩余战力判定。"
        );
        self.gfx.text(body, x0 + 20.0, y0 + 46.0, pw - 40.0, ph - 100.0, Font::Small, TEXT);
        self.button([x0 + pw / 2.0 - 70.0, y0 + ph - 46.0, x0 + pw / 2.0 + 70.0, y0 + ph - 12.0], t!("閉じる", "Close", "关闭"), Act::Help, false);
    }

    fn render_over(&mut self) {
        let (w, h) = (self.gfx.width as f32, self.gfx.height as f32);
        let pw = (w - 40.0).min(520.0);
        let ph = 250.0;
        let (x0, y0) = ((w - pw) / 2.0, (h - ph) / 2.0);
        self.panel([x0, y0, x0 + pw, y0 + ph]);
        let wd = &self.world;
        let (title, col) = match wd.winner {
            Some(0) => (t!("勝利", "Victory", "胜利"), GOLD),
            Some(_) => (t!("敗北", "Defeat", "失败"), RED),
            None => (t!("引き分け", "Draw", "平局"), TEXT),
        };
        let reason = match (wd.reason, wd.winner) {
            (Some(EndReason::Capture), Some(0)) => t!("敵の基地を占領した", "You captured the enemy base", "占领了敌方基地"),
            (Some(EndReason::Capture), _) => t!("基地を占領された", "Your base was captured", "基地被占领"),
            (Some(EndReason::Annihilation), Some(0)) => t!("敵の戦車をすべて撃破した", "All enemy tanks destroyed", "击毁了敌方全部坦克"),
            (Some(EndReason::Annihilation), Some(_)) => t!("自軍の戦車が全滅した", "All of your tanks were destroyed", "我方坦克全军覆没"),
            (Some(EndReason::Annihilation), None) => t!("双方が全滅した", "Both sides were wiped out", "双方全军覆没"),
            (Some(EndReason::TimeUp), _) => t!("時間切れ：残存戦力で判定", "Time up: decided by remaining strength", "时间到：按剩余战力判定"),
            (None, _) => "",
        };
        let lost = wd.units.iter().filter(|u| u.side == 0 && u.kind == Kind::Tank && !u.alive).count();
        let kills = wd.units.iter().filter(|u| u.side == 1 && u.kind == Kind::Tank && !u.alive).count();
        let decoys_hit = wd.units.iter().filter(|u| u.side == 0 && u.kind == Kind::Decoy && !u.alive).count();
        let t = wd.time as u32;
        let stats = tf!("撃破 {}両　損失 {}両　敵を欺いたデコイ {}　戦闘時間 {}:{:02}", "Kills {}  Losses {}  Decoys that fooled the enemy {}  Time {}:{:02}", "击毁 {}辆　损失 {}辆　骗过敌人的诱饵 {}　战斗时间 {}:{:02}", kills, lost, decoys_hit, t / 60, t % 60);
        self.gfx.text_shadow(title, x0, y0 + 14.0, pw, 56.0, Font::Title, col);
        self.gfx.text(reason, x0, y0 + 80.0, pw, 26.0, Font::BigCenter, TEXT);
        self.gfx.text(&stats, x0 + 20.0, y0 + 116.0, pw - 40.0, 50.0, Font::NormalCenter, DIM);
        let bw = (pw - 60.0) / 2.0;
        self.button([x0 + 20.0, y0 + ph - 56.0, x0 + 20.0 + bw, y0 + ph - 16.0], t!("新しい地形で再戦", "Rematch (new terrain)", "新地形再战"), Act::Again, true);
        self.button([x0 + 40.0 + bw, y0 + ph - 56.0, x0 + pw - 20.0, y0 + ph - 16.0], t!("タイトルへ", "Title", "返回标题"), Act::ToTitle, false);
    }
}

// ------------------------------------------------------------ 戦車の絵

fn ghost_unit(n: Nation, i: usize, pos: (f32, f32)) -> Unit {
    World::new_unit(0, Kind::Tank, n, i, pos)
}

/// 向きのある長方形（中心 c、角度 a、前後のずれ off、半長 hl、半幅 hw）
fn quad(g: &mut Gfx, c: (f32, f32), a: f32, off: f32, hl: f32, hw: f32, col: C) {
    let (ca, sa) = (a.cos(), a.sin());
    let cx = c.0 + ca * off;
    let cy = c.1 + sa * off;
    let p = |l: f32, w: f32| (cx + ca * l - sa * w, cy + sa * l + ca * w);
    let (a1, b1, c1, d1) = (p(hl, hw), p(hl, -hw), p(-hl, -hw), p(-hl, hw));
    g.tri(a1, b1, c1, col);
    g.tri(a1, c1, d1, col);
}

fn draw_tank(g: &mut Gfx, u: &Unit, alpha: f32, clock: f32) {
    let s: &TankSpec = u.spec();
    let (l, wd) = (s.len * TS, s.wid * TS);
    let dead = !u.alive;
    let k = if dead { 0.25 } else { 1.0 };
    let hull = with_a(shade(u.nation.hull(), k), alpha);
    let camo = with_a(shade(u.nation.camo(), k), alpha);
    let dark = with_a(shade([0.16, 0.15, 0.13, 1.0], if dead { 0.6 } else { 1.0 }), alpha);
    let p = u.pos;
    // 陣営の印（足元の円）
    if !dead {
        g.circle(p.0, p.1, l * 0.62, 20, with_a(side_color(u.side), 0.22 * alpha));
    }
    // 影
    quad(g, (p.0 + 3.0, p.1 + 4.0), u.hull, 0.0, l / 2.0, wd / 2.0, [0.0, 0.0, 0.0, 0.3 * alpha]);
    // 履帯
    let (ca, sa) = (u.hull.cos(), u.hull.sin());
    let side_off = wd / 2.0 - wd * 0.13;
    for sgn in [-1.0, 1.0] {
        let c = (p.0 - sa * side_off * sgn, p.1 + ca * side_off * sgn);
        quad(g, c, u.hull, 0.0, l / 2.0, wd * 0.13, dark);
    }
    // 車体
    quad(g, p, u.hull, 0.0, l * 0.46, wd * 0.33, hull);
    quad(g, p, u.hull, -l * 0.18, l * 0.12, wd * 0.2, camo);
    // 機関室
    quad(g, p, u.hull, -l * 0.36, l * 0.07, wd * 0.24, with_a(shade(u.nation.hull(), 0.75 * k), alpha));
    let bar_w = (2.0 + s.dmg / 20.0).min(5.0);
    if s.turret {
        let tc = (p.0 - ca * l * 0.04, p.1 - sa * l * 0.04);
        let tcol = with_a(shade(u.nation.hull(), 0.85 * k), alpha);
        let blen = l * s.barrel();
        let (tc2, ts2) = (u.turret.cos(), u.turret.sin());
        let start = (tc.0 + tc2 * l * 0.12, tc.1 + ts2 * l * 0.12);
        let end = (tc.0 + tc2 * (l * 0.12 + blen), tc.1 + ts2 * (l * 0.12 + blen));
        g.line(start, end, bar_w, dark);
        quad(g, tc, u.turret, 0.0, l * 0.2, wd * 0.24, tcol);
        quad(g, tc, u.turret, -l * 0.04, l * 0.07, wd * 0.12, camo);
        // 記章
        let f = u.nation.flag();
        g.circle(tc.0 - tc2 * l * 0.08, tc.1 - ts2 * l * 0.08, wd * 0.07, 8, with_a(f[1], alpha * k));
    } else {
        // 突撃砲：戦闘室と固定の砲
        let blen = l * s.barrel();
        let a = u.turret;
        let start = (p.0 + ca * l * 0.2, p.1 + sa * l * 0.2);
        g.line(start, (start.0 + a.cos() * blen, start.1 + a.sin() * blen), bar_w, dark);
        quad(g, p, u.hull, l * 0.02, l * 0.24, wd * 0.28, with_a(shade(u.nation.hull(), 0.85 * k), alpha));
    }
    if dead {
        // 燃える残骸の煙
        let t = u.dead_t;
        let n = 3;
        for i in 0..n {
            let ph = (clock * 0.6 + i as f32 / n as f32).fract();
            let r = TS * (0.2 + ph * 0.5);
            let a = (0.35 * (1.0 - ph)) * (1.0 - (t / 60.0).min(0.8));
            g.circle(p.0 + ph * TS * 0.6, p.1 - ph * TS * 1.2, r, 12, [0.2, 0.2, 0.2, a]);
        }
        if t < 8.0 {
            let fl = 0.5 + 0.5 * (clock * 12.0).sin();
            g.circle(p.0, p.1, TS * 0.18 * (1.0 + fl * 0.3), 8, [1.0, 0.5, 0.1, 0.7 * (1.0 - t / 8.0)]);
        }
    }
}

fn draw_decoy_wreck(g: &mut Gfx, u: &Unit) {
    let s = u.spec();
    let (l, wd) = (s.len * TS, s.wid * TS);
    quad(g, u.pos, u.hull + 0.3, 0.0, l * 0.4, wd * 0.2, [0.35, 0.30, 0.22, 0.7]);
    quad(g, u.pos, u.hull - 0.5, l * 0.1, l * 0.3, wd * 0.12, [0.30, 0.26, 0.20, 0.7]);
}

fn draw_drone(g: &mut Gfx, u: &Unit, clock: f32) {
    let p = u.pos;
    let r = TS * 0.32;
    let col = side_color(u.side);
    // 影
    g.circle(p.0 + 10.0, p.1 + 16.0, r * 0.9, 12, [0.0, 0.0, 0.0, 0.25]);
    let a = u.hull + std::f32::consts::FRAC_PI_4;
    for k in 0..4 {
        let aa = a + k as f32 * std::f32::consts::FRAC_PI_2;
        let tip = (p.0 + aa.cos() * r, p.1 + aa.sin() * r);
        g.line(p, tip, 3.0, [0.18, 0.18, 0.18, 1.0]);
        g.circle(tip.0, tip.1, r * 0.42, 10, [0.85, 0.85, 0.85, 0.35]);
        let spin = clock * 40.0 + k as f32;
        g.line((tip.0 - spin.cos() * r * 0.4, tip.1 - spin.sin() * r * 0.4), (tip.0 + spin.cos() * r * 0.4, tip.1 + spin.sin() * r * 0.4), 1.5, [0.1, 0.1, 0.1, 0.8]);
    }
    g.circle(p.0, p.1, r * 0.35, 10, col);
    g.ring(p.0, p.1, r * 0.35, 1.5, 10, [0.1, 0.1, 0.1, 1.0]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghost_units_exist_for_every_tank() {
        for (n, i) in crate::data::all_tanks() {
            let u = ghost_unit(n, i, (100.0, 100.0));
            assert_eq!(u.spec().name, n.tanks()[i].name);
        }
    }
}
