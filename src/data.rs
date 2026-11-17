//! 国と戦車の諸元、地形の性質
//!
//! 数値はゲーム向けに丸めたもの。装甲・貫通力はミリ、射程・視界・速度はマス（1マス≒25m）単位。
//! ドイツ・ソ連の重戦車は強力だが高価、日本・イタリアの戦車は弱いが安い。

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nation {
    Germany,
    Ussr,
    Usa,
    Uk,
    Japan,
    Italy,
}

pub const NATIONS: [Nation; 6] = [Nation::Germany, Nation::Ussr, Nation::Usa, Nation::Uk, Nation::Japan, Nation::Italy];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weight {
    Light,
    Medium,
    Heavy,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Standard,
    Minor,
    Spg,
    Proto,
}

impl Role {
    pub fn tag(self) -> &'static str {
        match self {
            Role::Standard => "",
            Role::Minor => t!("特殊", "Minor", "特殊"),
            Role::Spg => t!("自走砲", "SPG", "自行火炮"),
            Role::Proto => t!("試作", "Proto", "原型"),
        }
    }
}

/// 戦車の諸元
pub struct TankSpec {
    pub name: [&'static str; 3],
    pub cost: u32,
    pub hp: f32,
    /// 正面・側面装甲（背面は側面の7割）
    pub front: f32,
    pub side: f32,
    /// 至近距離での貫通力
    pub pen: f32,
    pub dmg: f32,
    /// 装填（秒）
    pub reload: f32,
    pub range: f32,
    /// 平地での速度（マス/秒）
    pub speed: f32,
    pub sight: f32,
    pub weight: Weight,
    pub role: Role,
    /// 描画の寸法（マス）
    pub len: f32,
    pub wid: f32,
    /// 旋回砲塔があるか（無ければ車体を向けて撃つ）
    pub turret: bool,
    pub gun: &'static str,
}

impl TankSpec {
    pub fn name(&self) -> &'static str {
        t!(self.name[0], self.name[1], self.name[2])
    }
    pub fn rear(&self) -> f32 {
        self.side * 0.7
    }
    /// 砲身の長さ（車体長に対する割合）
    pub fn barrel(&self) -> f32 {
        (0.35 + self.pen / 400.0).min(0.85)
    }
}

macro_rules! tank {
    ($ja:expr, $en:expr, $zh:expr, cost $c:expr, hp $hp:expr, armor $f:expr, $s:expr, pen $p:expr, dmg $d:expr, reload $r:expr, range $rg:expr, speed $sp:expr, sight $si:expr, $w:ident, $role:ident, size $l:expr, $wd:expr, turret $t:expr, gun $g:expr) => {
        TankSpec { name: [$ja, $en, $zh], cost: $c, hp: $hp, front: $f, side: $s, pen: $p, dmg: $d, reload: $r, range: $rg, speed: $sp, sight: $si, weight: Weight::$w, role: Role::$role, len: $l, wid: $wd, turret: $t, gun: $g }
    };
}

static GERMANY: [TankSpec; 7] = [
    tank!("III号戦車J型", "Panzer III J", "三号坦克J型", cost 260, hp 90.0, armor 50.0, 30.0, pen 65.0, dmg 24.0, reload 2.8, range 13.0, speed 1.9, sight 13.0, Medium, Standard, size 0.86, 0.52, turret true, gun "5cm"),
    tank!("IV号戦車H型", "Panzer IV H", "四号坦克H型", cost 380, hp 100.0, armor 80.0, 30.0, pen 110.0, dmg 34.0, reload 3.4, range 15.0, speed 1.7, sight 13.0, Medium, Standard, size 0.92, 0.54, turret true, gun "7.5cm L/48"),
    tank!("パンター G型", "Panther G", "黑豹G型", cost 650, hp 125.0, armor 120.0, 45.0, pen 150.0, dmg 38.0, reload 3.8, range 17.0, speed 1.9, sight 14.0, Medium, Standard, size 1.0, 0.6, turret true, gun "7.5cm L/70"),
    tank!("ティーガーI", "Tiger I", "虎式I型", cost 820, hp 155.0, armor 110.0, 80.0, pen 145.0, dmg 50.0, reload 4.6, range 16.0, speed 1.3, sight 14.0, Heavy, Standard, size 1.02, 0.66, turret true, gun "8.8cm"),
    tank!("II号L型 ルクス", "Panzer II L Luchs", "二号L型山猫", cost 115, hp 58.0, armor 30.0, 20.0, pen 38.0, dmg 11.0, reload 0.9, range 10.0, speed 2.8, sight 15.0, Light, Minor, size 0.72, 0.44, turret true, gun "2cm"),
    tank!("III号突撃砲G型", "StuG III G", "三号突击炮G型", cost 330, hp 92.0, armor 80.0, 30.0, pen 112.0, dmg 34.0, reload 3.1, range 15.0, speed 1.8, sight 12.0, Medium, Spg, size 0.86, 0.52, turret false, gun "7.5cm StuK"),
    tank!("VIII号戦車 マウス", "Panzer VIII Maus", "鼠式超重坦克", cost 1350, hp 260.0, armor 220.0, 185.0, pen 185.0, dmg 78.0, reload 6.8, range 17.0, speed 0.75, sight 13.0, Heavy, Proto, size 1.24, 0.76, turret true, gun "12.8cm"),
];

static USSR: [TankSpec; 7] = [
    tank!("T-70", "T-70", "T-70", cost 130, hp 65.0, armor 45.0, 15.0, pen 45.0, dmg 20.0, reload 2.5, range 11.0, speed 2.1, sight 12.0, Light, Standard, size 0.74, 0.46, turret true, gun "45mm"),
    tank!("T-34/76", "T-34/76", "T-34/76", cost 300, hp 100.0, armor 70.0, 45.0, pen 80.0, dmg 32.0, reload 3.2, range 14.0, speed 2.1, sight 12.0, Medium, Standard, size 0.9, 0.56, turret true, gun "76mm"),
    tank!("T-34/85", "T-34/85", "T-34/85", cost 420, hp 105.0, armor 90.0, 45.0, pen 120.0, dmg 38.0, reload 3.7, range 15.0, speed 2.0, sight 13.0, Medium, Standard, size 0.94, 0.56, turret true, gun "85mm"),
    tank!("IS-2", "IS-2", "IS-2", cost 780, hp 155.0, armor 130.0, 90.0, pen 165.0, dmg 72.0, reload 7.5, range 16.0, speed 1.4, sight 13.0, Heavy, Standard, size 1.04, 0.64, turret true, gun "122mm"),
    tank!("T-35", "T-35", "T-35", cost 240, hp 130.0, armor 35.0, 25.0, pen 52.0, dmg 24.0, reload 2.7, range 12.0, speed 1.2, sight 13.0, Heavy, Minor, size 1.16, 0.60, turret true, gun "76mm+45mm"),
    tank!("ISU-152", "ISU-152", "ISU-152", cost 640, hp 135.0, armor 110.0, 75.0, pen 155.0, dmg 92.0, reload 8.5, range 17.0, speed 1.3, sight 12.0, Heavy, Spg, size 1.04, 0.64, turret false, gun "152mm"),
    tank!("KV-5", "KV-5", "KV-5", cost 1150, hp 225.0, armor 180.0, 150.0, pen 155.0, dmg 58.0, reload 5.2, range 16.0, speed 1.1, sight 13.0, Heavy, Proto, size 1.18, 0.72, turret true, gun "107mm"),
];

static USA: [TankSpec; 7] = [
    tank!("M5 スチュアート", "M5 Stuart", "M5斯图亚特", cost 150, hp 70.0, armor 45.0, 28.0, pen 53.0, dmg 18.0, reload 2.0, range 11.0, speed 2.5, sight 14.0, Light, Standard, size 0.76, 0.48, turret true, gun "37mm"),
    tank!("M4 シャーマン", "M4 Sherman", "M4谢尔曼", cost 320, hp 100.0, armor 76.0, 38.0, pen 88.0, dmg 32.0, reload 3.0, range 14.0, speed 1.9, sight 13.0, Medium, Standard, size 0.9, 0.56, turret true, gun "75mm"),
    tank!("M4A3E8 イージーエイト", "M4A3E8 Easy Eight", "M4A3E8", cost 420, hp 105.0, armor 90.0, 38.0, pen 120.0, dmg 34.0, reload 3.2, range 15.0, speed 2.0, sight 13.0, Medium, Standard, size 0.94, 0.58, turret true, gun "76mm"),
    tank!("M26 パーシング", "M26 Pershing", "M26潘兴", cost 700, hp 135.0, armor 110.0, 76.0, pen 150.0, dmg 46.0, reload 4.2, range 16.0, speed 1.6, sight 13.0, Heavy, Standard, size 1.0, 0.62, turret true, gun "90mm"),
    tank!("M22 ローカスト", "M22 Locust", "M22蝗虫", cost 110, hp 55.0, armor 25.0, 13.0, pen 50.0, dmg 17.0, reload 2.0, range 11.0, speed 2.7, sight 14.0, Light, Minor, size 0.68, 0.42, turret true, gun "37mm"),
    tank!("M7 プリースト", "M7 Priest", "M7牧师", cost 370, hp 75.0, armor 45.0, 25.0, pen 82.0, dmg 58.0, reload 5.6, range 19.0, speed 1.7, sight 12.0, Medium, Spg, size 0.90, 0.56, turret false, gun "105mm"),
    tank!("T28 超重戦車", "T28 Super Heavy", "T28超重坦克", cost 1080, hp 210.0, armor 250.0, 130.0, pen 180.0, dmg 64.0, reload 5.4, range 17.0, speed 0.7, sight 13.0, Heavy, Proto, size 1.18, 0.80, turret false, gun "105mm T5"),
];

static UK: [TankSpec; 7] = [
    tank!("クルセーダーIII", "Crusader III", "十字军III", cost 190, hp 75.0, armor 50.0, 28.0, pen 72.0, dmg 24.0, reload 2.5, range 12.0, speed 2.4, sight 13.0, Light, Standard, size 0.88, 0.5, turret true, gun "6pdr"),
    tank!("クロムウェル", "Cromwell", "克伦威尔", cost 300, hp 95.0, armor 76.0, 50.0, pen 80.0, dmg 30.0, reload 2.9, range 14.0, speed 2.5, sight 13.0, Medium, Standard, size 0.9, 0.56, turret true, gun "75mm"),
    tank!("チャーチルVII", "Churchill VII", "丘吉尔VII", cost 460, hp 145.0, armor 150.0, 95.0, pen 80.0, dmg 30.0, reload 3.1, range 14.0, speed 1.0, sight 12.0, Heavy, Standard, size 1.06, 0.58, turret true, gun "75mm"),
    tank!("シャーマン・ファイアフライ", "Sherman Firefly", "萤火虫", cost 480, hp 100.0, armor 76.0, 38.0, pen 155.0, dmg 42.0, reload 4.2, range 17.0, speed 1.8, sight 13.0, Medium, Standard, size 0.94, 0.56, turret true, gun "17pdr"),
    tank!("テトラーク軽戦車", "Tetrarch Mk VII", "领主轻型坦克", cost 105, hp 55.0, armor 16.0, 14.0, pen 46.0, dmg 18.0, reload 2.2, range 11.0, speed 2.7, sight 14.0, Light, Minor, size 0.70, 0.44, turret true, gun "2pdr"),
    tank!("ビショップ自走砲", "Bishop SPG", "主教自行火炮", cost 310, hp 80.0, armor 60.0, 45.0, pen 74.0, dmg 48.0, reload 4.8, range 18.0, speed 1.2, sight 12.0, Medium, Spg, size 0.86, 0.54, turret false, gun "25pdr"),
    tank!("TOG II*", "TOG II*", "TOG II*", cost 720, hp 200.0, armor 115.0, 76.0, pen 140.0, dmg 40.0, reload 3.6, range 16.0, speed 0.8, sight 13.0, Heavy, Proto, size 1.30, 0.62, turret true, gun "17pdr"),
];

static JAPAN: [TankSpec; 7] = [
    tank!("九五式軽戦車 ハ号", "Type 95 Ha-Go", "九五式轻战车", cost 65, hp 50.0, armor 12.0, 12.0, pen 38.0, dmg 16.0, reload 2.0, range 10.0, speed 2.5, sight 13.0, Light, Standard, size 0.72, 0.44, turret true, gun "37mm"),
    tank!("九七式改 新砲塔チハ", "Type 97 Shinhoto Chi-Ha", "九七式改中战车", cost 105, hp 65.0, armor 25.0, 22.0, pen 50.0, dmg 20.0, reload 2.4, range 11.0, speed 2.1, sight 12.0, Medium, Standard, size 0.8, 0.5, turret true, gun "47mm"),
    tank!("一式中戦車 チヘ", "Type 1 Chi-He", "一式中战车", cost 150, hp 75.0, armor 50.0, 25.0, pen 55.0, dmg 22.0, reload 2.4, range 12.0, speed 2.2, sight 12.0, Medium, Standard, size 0.84, 0.52, turret true, gun "47mm"),
    tank!("三式中戦車 チヌ", "Type 3 Chi-Nu", "三式中战车", cost 220, hp 85.0, armor 50.0, 25.0, pen 80.0, dmg 32.0, reload 3.5, range 13.0, speed 1.9, sight 12.0, Medium, Standard, size 0.88, 0.54, turret true, gun "75mm"),
    tank!("特二式内火艇 カミ", "Type 2 Ka-Mi", "特二式内火艇", cost 90, hp 60.0, armor 12.0, 10.0, pen 40.0, dmg 16.0, reload 2.1, range 10.0, speed 2.2, sight 13.0, Light, Minor, size 0.94, 0.48, turret true, gun "37mm"),
    tank!("四式自走砲 ホロ", "Type 4 Ho-Ro", "四式自行火炮", cost 280, hp 65.0, armor 25.0, 12.0, pen 95.0, dmg 78.0, reload 7.8, range 18.0, speed 1.7, sight 12.0, Medium, Spg, size 0.84, 0.52, turret false, gun "150mm"),
    tank!("オイ車", "O-I Super-Heavy", "O-I超重坦克", cost 540, hp 175.0, armor 110.0, 70.0, pen 115.0, dmg 56.0, reload 5.5, range 15.0, speed 0.95, sight 13.0, Heavy, Proto, size 1.20, 0.72, turret true, gun "105mm"),
];

static ITALY: [TankSpec; 7] = [
    tank!("L6/40", "L6/40", "L6/40", cost 60, hp 50.0, armor 30.0, 15.0, pen 30.0, dmg 9.0, reload 0.8, range 9.0, speed 2.3, sight 13.0, Light, Standard, size 0.7, 0.44, turret true, gun "20mm"),
    tank!("M13/40", "M13/40", "M13/40", cost 100, hp 65.0, armor 30.0, 25.0, pen 48.0, dmg 20.0, reload 2.5, range 11.0, speed 1.7, sight 12.0, Medium, Standard, size 0.8, 0.5, turret true, gun "47mm"),
    tank!("セモヴェンテ 75/18", "Semovente 75/18", "75/18自行火炮", cost 160, hp 75.0, armor 50.0, 25.0, pen 75.0, dmg 34.0, reload 3.6, range 13.0, speed 1.8, sight 11.0, Medium, Spg, size 0.84, 0.54, turret false, gun "75mm"),
    tank!("P26/40", "P26/40", "P26/40", cost 230, hp 90.0, armor 50.0, 40.0, pen 82.0, dmg 32.0, reload 3.3, range 13.0, speed 1.8, sight 12.0, Medium, Standard, size 0.9, 0.56, turret true, gun "75mm"),
    tank!("L3/33 豆戦車", "L3/33 Tankette", "L3/33超轻型坦克", cost 45, hp 38.0, armor 14.0, 9.0, pen 22.0, dmg 6.0, reload 0.55, range 8.0, speed 2.7, sight 13.0, Light, Minor, size 0.58, 0.36, turret false, gun "8mmx2"),
    tank!("セモヴェンテ 90/53", "Semovente 90/53", "90/53自行火炮", cost 310, hp 68.0, armor 30.0, 15.0, pen 142.0, dmg 48.0, reload 4.5, range 18.0, speed 1.6, sight 13.0, Medium, Spg, size 0.86, 0.52, turret false, gun "90mm"),
    tank!("P43 bis", "P43 bis", "P43 bis", cost 460, hp 115.0, armor 100.0, 55.0, pen 125.0, dmg 44.0, reload 3.9, range 15.0, speed 1.7, sight 13.0, Heavy, Proto, size 0.96, 0.58, turret true, gun "90mm"),
];

impl Nation {
    pub fn index(self) -> usize {
        match self {
            Nation::Germany => 0,
            Nation::Ussr => 1,
            Nation::Usa => 2,
            Nation::Uk => 3,
            Nation::Japan => 4,
            Nation::Italy => 5,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Nation::Germany => t!("ドイツ", "Germany", "德国"),
            Nation::Ussr => t!("ソ連", "Soviet Union", "苏联"),
            Nation::Usa => t!("アメリカ", "United States", "美国"),
            Nation::Uk => t!("イギリス", "United Kingdom", "英国"),
            Nation::Japan => t!("日本", "Japan", "日本"),
            Nation::Italy => t!("イタリア", "Italy", "意大利"),
        }
    }
    /// 国の特色の説明
    pub fn blurb(self) -> &'static str {
        match self {
            Nation::Germany => t!("ティーガーとパンター。少数精鋭の重装甲と長砲身", "Tiger and Panther: few but heavily armoured, with long guns", "虎式与黑豹：少而精，重甲长炮"),
            Nation::Ussr => t!("T-34とIS-2。機動力のある中戦車と強力な重戦車", "T-34 and IS-2: mobile mediums and a hard-hitting heavy", "T-34与IS-2：机动的中型坦克与强力重坦"),
            Nation::Usa => t!("シャーマンの数と信頼性。パーシングが切り札", "Many reliable Shermans, with the Pershing as a trump card", "数量众多的谢尔曼，潘兴为王牌"),
            Nation::Uk => t!("重装甲の歩兵戦車と、17ポンド砲のファイアフライ", "Heavily armoured infantry tanks and the 17-pdr Firefly", "重甲步兵坦克与17磅炮萤火虫"),
            Nation::Japan => t!("安価で軽快な戦車を大量に。数と速さで基地を突く", "Cheap, nimble tanks in numbers: strike the base with speed", "廉价轻快的坦克大量投入，以数量和速度突袭基地"),
            Nation::Italy => t!("安価な戦車と突撃砲。デコイと竜の歯で補う", "Cheap tanks and assault guns: make up for it with decoys and teeth", "廉价坦克与突击炮，以诱饵和龙牙弥补"),
        }
    }
    pub fn tanks(self) -> &'static [TankSpec] {
        match self {
            Nation::Germany => &GERMANY,
            Nation::Ussr => &USSR,
            Nation::Usa => &USA,
            Nation::Uk => &UK,
            Nation::Japan => &JAPAN,
            Nation::Italy => &ITALY,
        }
    }
    /// 車体の色
    pub fn hull(self) -> [f32; 4] {
        match self {
            Nation::Germany => [0.62, 0.56, 0.38, 1.0],
            Nation::Ussr => [0.33, 0.40, 0.22, 1.0],
            Nation::Usa => [0.38, 0.40, 0.26, 1.0],
            Nation::Uk => [0.50, 0.48, 0.34, 1.0],
            Nation::Japan => [0.52, 0.47, 0.30, 1.0],
            Nation::Italy => [0.74, 0.66, 0.46, 1.0],
        }
    }
    /// 迷彩の色
    pub fn camo(self) -> [f32; 4] {
        match self {
            Nation::Germany => [0.30, 0.36, 0.22, 1.0],
            Nation::Ussr => [0.27, 0.33, 0.18, 1.0],
            Nation::Usa => [0.30, 0.32, 0.20, 1.0],
            Nation::Uk => [0.34, 0.38, 0.28, 1.0],
            Nation::Japan => [0.30, 0.38, 0.22, 1.0],
            Nation::Italy => [0.55, 0.45, 0.28, 1.0],
        }
    }
    /// 旗（横三分割の色。単色国は同じ色を並べる）と記章の色
    pub fn flag(self) -> [[f32; 4]; 3] {
        match self {
            Nation::Germany => [[0.1, 0.1, 0.1, 1.0], [0.85, 0.85, 0.85, 1.0], [0.75, 0.12, 0.12, 1.0]],
            Nation::Ussr => [[0.78, 0.1, 0.1, 1.0], [0.78, 0.1, 0.1, 1.0], [0.78, 0.1, 0.1, 1.0]],
            Nation::Usa => [[0.15, 0.2, 0.45, 1.0], [0.92, 0.92, 0.92, 1.0], [0.75, 0.12, 0.15, 1.0]],
            Nation::Uk => [[0.12, 0.2, 0.45, 1.0], [0.92, 0.92, 0.92, 1.0], [0.75, 0.12, 0.15, 1.0]],
            Nation::Japan => [[0.95, 0.95, 0.95, 1.0], [0.8, 0.1, 0.12, 1.0], [0.95, 0.95, 0.95, 1.0]],
            Nation::Italy => [[0.1, 0.55, 0.3, 1.0], [0.95, 0.95, 0.95, 1.0], [0.8, 0.15, 0.15, 1.0]],
        }
    }
    #[cfg(test)]
    pub fn cheapest(self) -> u32 {
        self.tanks().iter().map(|t| t.cost).min().unwrap_or(100)
    }
}

/// 予算規模
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Budget {
    Small,
    Medium,
    Large,
}

pub const BUDGETS: [Budget; 3] = [Budget::Small, Budget::Medium, Budget::Large];

impl Budget {
    pub fn points(self) -> u32 {
        match self {
            Budget::Small => 1500,
            Budget::Medium => 3000,
            Budget::Large => 5500,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Budget::Small => t!("小", "Small", "小"),
            Budget::Medium => t!("中", "Medium", "中"),
            Budget::Large => t!("大", "Large", "大"),
        }
    }
}

/// 全ての国の戦車（国, 番号）
pub fn all_tanks() -> impl Iterator<Item = (Nation, usize)> {
    NATIONS.into_iter().flat_map(|n| (0..n.tanks().len()).map(move |i| (n, i)))
}

pub const DECOY_COST: u32 = 30;
pub const TEETH_COST: u32 = 20;
pub const TEETH_HP: f32 = 220.0;

/// 地形
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Terrain {
    Plain,
    Road,
    Forest,
    Hill,
    Swamp,
    Town,
    Water,
    Ford,
    Bridge,
    /// 山岳（戦車は通れず、視線を遮る）
    Mountain,
    /// 砂地（やや遅い）
    Sand,
    /// 生垣（遅く、視線を遮る）
    Bocage,
}

impl Terrain {
    /// 進撃速度の倍率（重い戦車ほど悪路で遅い）。0 は通れない
    pub fn speed(self, w: Weight) -> f32 {
        let (l, m, h) = match self {
            Terrain::Plain => (1.0, 1.0, 1.0),
            Terrain::Road | Terrain::Bridge => (1.35, 1.35, 1.3),
            Terrain::Forest => (0.55, 0.45, 0.32),
            Terrain::Hill => (0.8, 0.68, 0.52),
            Terrain::Swamp => (0.42, 0.3, 0.17),
            Terrain::Town => (0.7, 0.62, 0.5),
            Terrain::Ford => (0.42, 0.36, 0.28),
            Terrain::Sand => (0.85, 0.78, 0.66),
            Terrain::Bocage => (0.38, 0.32, 0.24),
            Terrain::Water | Terrain::Mountain => (0.0, 0.0, 0.0),
        };
        match w {
            Weight::Light => l,
            Weight::Medium => m,
            Weight::Heavy => h,
        }
    }
    pub fn passable(self) -> bool {
        !matches!(self, Terrain::Water | Terrain::Mountain)
    }
    /// 中にいる戦車を隠す（見つけにくく、当たりにくい）
    pub fn cover(self) -> bool {
        matches!(self, Terrain::Forest | Terrain::Town | Terrain::Bocage)
    }
    /// 視線を遮る度合い
    pub fn block(self) -> u32 {
        match self {
            Terrain::Forest => 2,
            Terrain::Town => 2,
            Terrain::Bocage => 2,
            Terrain::Mountain => 9,
            _ => 0,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Terrain::Plain => t!("平地", "Open ground", "平地"),
            Terrain::Road => t!("道路", "Road", "道路"),
            Terrain::Forest => t!("森", "Forest", "森林"),
            Terrain::Hill => t!("丘", "Hill", "丘陵"),
            Terrain::Swamp => t!("湿地", "Marsh", "沼泽"),
            Terrain::Town => t!("村落", "Village", "村落"),
            Terrain::Water => t!("川", "River", "河流"),
            Terrain::Ford => t!("浅瀬", "Ford", "浅滩"),
            Terrain::Bridge => t!("橋", "Bridge", "桥"),
            Terrain::Mountain => t!("山岳", "Mountains", "山岳"),
            Terrain::Sand => t!("砂地", "Sand", "沙地"),
            Terrain::Bocage => t!("生垣", "Hedgerow", "树篱"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heavy_tanks_cost_more_than_axis_minor_tanks() {
        let max = |n: Nation| n.tanks().iter().map(|t| t.cost).max().unwrap();
        for strong in [Nation::Germany, Nation::Ussr] {
            for cheap in [Nation::Japan, Nation::Italy] {
                assert!(max(strong) > 2 * max(cheap));
                assert!(strong.cheapest() > cheap.cheapest());
            }
        }
    }

    #[test]
    fn terrain_slows_heavier_tanks_more() {
        for t in [Terrain::Forest, Terrain::Swamp, Terrain::Hill, Terrain::Town] {
            assert!(t.speed(Weight::Heavy) < t.speed(Weight::Light));
            assert!(t.speed(Weight::Medium) < Terrain::Plain.speed(Weight::Medium));
        }
        assert_eq!(Terrain::Water.speed(Weight::Light), 0.0);
    }
}
