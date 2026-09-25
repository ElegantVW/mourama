//! Pure formulas: production, costs, combat, bronze. No I/O.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_SEATS: usize = 5;
pub const MAP_RADIUS: i32 = 5;
pub const SEAT_RING: i32 = 4;
pub const WORLD_SPEED: f64 = 1.0;
pub const SECONDS_PER_HEX: f64 = 60.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Resources {
    pub cobre: f64,
    pub estanho: f64,
    pub seara: f64,
    pub orvalho: f64,
}

impl Resources {
    pub fn add(&mut self, other: Resources) {
        self.cobre += other.cobre;
        self.estanho += other.estanho;
        self.seara += other.seara;
        self.orvalho += other.orvalho;
    }

    pub fn saturating_sub(&mut self, other: Resources) -> bool {
        if self.cobre + 1e-9 < other.cobre
            || self.estanho + 1e-9 < other.estanho
            || self.seara + 1e-9 < other.seara
            || self.orvalho + 1e-9 < other.orvalho
        {
            return false;
        }
        self.cobre -= other.cobre;
        self.estanho -= other.estanho;
        self.seara -= other.seara;
        self.orvalho -= other.orvalho;
        true
    }

    pub fn bronze_available(&self) -> f64 {
        self.cobre.min(self.estanho)
    }

    /// Spend N bronze = N cobre + N estanho.
    pub fn spend_bronze(&mut self, n: f64) -> bool {
        if self.bronze_available() + 1e-9 < n {
            return false;
        }
        self.cobre -= n;
        self.estanho -= n;
        true
    }

    pub fn scale(self, f: f64) -> Resources {
        Resources {
            cobre: self.cobre * f,
            estanho: self.estanho * f,
            seara: self.seara * f,
            orvalho: self.orvalho * f,
        }
    }

    pub fn clamp_cap(&mut self, cap: f64) {
        self.cobre = self.cobre.min(cap);
        self.estanho = self.estanho.min(cap);
        self.seara = self.seara.min(cap);
        self.orvalho = self.orvalho.min(cap);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Building {
    Casa,
    Eira,
    Mina,
    Veio,
    Fonte,
    Curral,
    Muralha,
    Anta,
    Celeiro,
}

impl Building {
    pub fn all() -> &'static [Building] {
        &[
            Building::Casa,
            Building::Eira,
            Building::Mina,
            Building::Veio,
            Building::Fonte,
            Building::Curral,
            Building::Muralha,
            Building::Anta,
            Building::Celeiro,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Building::Casa => "casa",
            Building::Eira => "eira",
            Building::Mina => "mina",
            Building::Veio => "veio",
            Building::Fonte => "fonte",
            Building::Curral => "curral",
            Building::Muralha => "muralha",
            Building::Anta => "anta",
            Building::Celeiro => "celeiro",
        }
    }

    pub fn parse(s: &str) -> Option<Building> {
        Some(match s {
            "casa" => Building::Casa,
            "eira" => Building::Eira,
            "mina" => Building::Mina,
            "veio" => Building::Veio,
            "fonte" => Building::Fonte,
            "curral" => Building::Curral,
            "muralha" => Building::Muralha,
            "anta" => Building::Anta,
            "celeiro" => Building::Celeiro,
            _ => return None,
        })
    }

    pub fn title(self) -> &'static str {
        match self {
            Building::Casa => "Casa circular",
            Building::Eira => "Eira",
            Building::Mina => "Mina",
            Building::Veio => "Veio",
            Building::Fonte => "Fonte",
            Building::Curral => "Curral",
            Building::Muralha => "Muralha",
            Building::Anta => "Anta",
            Building::Celeiro => "Celeiro",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Pastor,
    Trasgo,
    Falcao,
    Javali,
    Guerreiro,
    Serpe,
}

impl Unit {
    pub fn all() -> &'static [Unit] {
        &[
            Unit::Pastor,
            Unit::Trasgo,
            Unit::Falcao,
            Unit::Javali,
            Unit::Guerreiro,
            Unit::Serpe,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Unit::Pastor => "pastor",
            Unit::Trasgo => "trasgo",
            Unit::Falcao => "falcao",
            Unit::Javali => "javali",
            Unit::Guerreiro => "guerreiro",
            Unit::Serpe => "serpe",
        }
    }

    pub fn parse(s: &str) -> Option<Unit> {
        Some(match s {
            "pastor" => Unit::Pastor,
            "trasgo" => Unit::Trasgo,
            "falcao" | "falcão" => Unit::Falcao,
            "javali" => Unit::Javali,
            "guerreiro" => Unit::Guerreiro,
            "serpe" => Unit::Serpe,
            _ => return None,
        })
    }

    pub fn title(self) -> &'static str {
        match self {
            Unit::Pastor => "Pastor",
            Unit::Trasgo => "Trasgo",
            Unit::Falcao => "Falcão",
            Unit::Javali => "Javali",
            Unit::Guerreiro => "Guerreiro de bronze",
            Unit::Serpe => "Serpe",
        }
    }

    pub fn atk(self) -> f64 {
        match self {
            Unit::Pastor => 10.0,
            Unit::Trasgo => 20.0,
            Unit::Falcao => 5.0,
            Unit::Javali => 40.0,
            Unit::Guerreiro => 55.0,
            Unit::Serpe => 15.0,
        }
    }

    pub fn def(self) -> f64 {
        match self {
            Unit::Pastor => 10.0,
            Unit::Trasgo => 8.0,
            Unit::Falcao => 2.0,
            Unit::Javali => 35.0,
            Unit::Guerreiro => 50.0,
            Unit::Serpe => 15.0,
        }
    }

    pub fn speed(self) -> f64 {
        match self {
            Unit::Pastor => 1.0,
            Unit::Trasgo => 1.4,
            Unit::Falcao => 2.2,
            Unit::Javali => 0.7,
            Unit::Guerreiro => 0.9,
            Unit::Serpe => 0.8,
        }
    }

    pub fn train_secs(self) -> i64 {
        match self {
            Unit::Pastor => 30,
            Unit::Trasgo => 45,
            Unit::Falcao => 40,
            Unit::Javali => 90,
            Unit::Guerreiro => 120,
            Unit::Serpe => 180,
        }
    }

    pub fn train_cost(self) -> (Resources, f64) {
        // (resources, bronze)
        match self {
            Unit::Pastor => (
                Resources {
                    seara: 50.0,
                    ..Resources::default()
                },
                0.0,
            ),
            Unit::Trasgo => (
                Resources {
                    cobre: 20.0,
                    orvalho: 40.0,
                    ..Resources::default()
                },
                0.0,
            ),
            Unit::Falcao => (
                Resources {
                    orvalho: 30.0,
                    ..Resources::default()
                },
                0.0,
            ),
            Unit::Javali => (
                Resources {
                    seara: 80.0,
                    cobre: 40.0,
                    ..Resources::default()
                },
                0.0,
            ),
            Unit::Guerreiro => (
                Resources {
                    seara: 40.0,
                    ..Resources::default()
                },
                30.0,
            ),
            Unit::Serpe => (
                Resources {
                    orvalho: 200.0,
                    ..Resources::default()
                },
                0.0,
            ),
        }
    }
}

pub type Units = BTreeMap<Unit, i64>;

pub fn units_from_json(v: &serde_json::Value) -> Units {
    let mut u = Units::new();
    if let Some(obj) = v.as_object() {
        for (k, n) in obj {
            if let Some(kind) = Unit::parse(k) {
                let c = n.as_i64().unwrap_or(0);
                if c > 0 {
                    u.insert(kind, c);
                }
            }
        }
    }
    u
}

pub fn units_to_json(u: &Units) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for (k, n) in u {
        if *n > 0 {
            obj.insert(k.as_str().to_string(), serde_json::json!(*n));
        }
    }
    serde_json::Value::Object(obj)
}

pub fn units_total(u: &Units) -> i64 {
    u.values().sum()
}

pub fn scale_units(u: &Units, keep: f64) -> Units {
    let mut out = Units::new();
    for (k, n) in u {
        let kept = ((*n as f64) * keep).round() as i64;
        if kept > 0 {
            out.insert(*k, kept);
        }
    }
    out
}

pub fn sub_units(have: &mut Units, take: &Units) -> bool {
    for (k, n) in take {
        if have.get(k).copied().unwrap_or(0) < *n {
            return false;
        }
    }
    for (k, n) in take {
        let left = have.get(k).copied().unwrap_or(0) - n;
        if left <= 0 {
            have.remove(k);
        } else {
            have.insert(*k, left);
        }
    }
    true
}

pub fn add_units(have: &mut Units, add: &Units) {
    for (k, n) in add {
        *have.entry(*k).or_insert(0) += n;
    }
}

pub fn slowest_speed(u: &Units) -> f64 {
    u.iter()
        .filter(|(_, n)| **n > 0)
        .map(|(k, _)| k.speed())
        .fold(f64::INFINITY, f64::min)
}

pub fn travel_secs(distance: i32, units: &Units) -> i64 {
    let dist = distance.max(1) as f64;
    let speed = slowest_speed(units);
    let speed = if speed.is_finite() && speed > 0.0 {
        speed
    } else {
        1.0
    };
    ((dist * SECONDS_PER_HEX) / (speed * WORLD_SPEED)).ceil() as i64
}

pub fn prod_per_hour(kind: Building, level: i32) -> f64 {
    if level <= 0 {
        return 0.0;
    }
    let base = match kind {
        Building::Eira => 40.0,
        Building::Mina => 30.0,
        Building::Veio => 30.0,
        Building::Fonte => 20.0,
        _ => 0.0,
    };
    base * 1.25_f64.powi(level - 1)
}

pub fn accrue(kind: Building, level: i32, secs: i64) -> f64 {
    prod_per_hour(kind, level) * (secs.max(0) as f64) / 3600.0
}

pub fn storage_cap(celeiro: i32) -> f64 {
    800.0 * 1.45_f64.powi(celeiro.max(0))
}

pub fn upgrade_secs(level_from: i32) -> i64 {
    (90.0 * 1.42_f64.powi(level_from) / WORLD_SPEED).ceil() as i64
}

pub fn upgrade_cost(kind: Building, level_from: i32) -> (Resources, f64) {
    let m = 1.55_f64.powi(level_from);
    match kind {
        Building::Casa => (
            Resources {
                cobre: 80.0 * m,
                estanho: 80.0 * m,
                seara: 60.0 * m,
                orvalho: 0.0,
            },
            0.0,
        ),
        Building::Eira => (
            Resources {
                seara: 50.0 * m,
                cobre: 30.0 * m,
                ..Resources::default()
            },
            0.0,
        ),
        Building::Mina => (
            Resources {
                cobre: 40.0 * m,
                seara: 40.0 * m,
                ..Resources::default()
            },
            0.0,
        ),
        Building::Veio => (
            Resources {
                estanho: 40.0 * m,
                seara: 40.0 * m,
                ..Resources::default()
            },
            0.0,
        ),
        Building::Fonte => (
            Resources {
                orvalho: 40.0 * m,
                seara: 30.0 * m,
                cobre: 20.0 * m,
                ..Resources::default()
            },
            0.0,
        ),
        Building::Curral => (
            Resources {
                seara: 60.0 * m,
                cobre: 40.0 * m,
                ..Resources::default()
            },
            0.0,
        ),
        Building::Muralha => (
            Resources {
                seara: 50.0 * m,
                ..Resources::default()
            },
            20.0 * m,
        ),
        Building::Anta => (
            Resources {
                orvalho: 80.0 * m,
                estanho: 40.0 * m,
                ..Resources::default()
            },
            0.0,
        ),
        Building::Celeiro => (
            Resources {
                seara: 70.0 * m,
                cobre: 30.0 * m,
                estanho: 30.0 * m,
                ..Resources::default()
            },
            0.0,
        ),
    }
}

pub fn attack_power(units: &Units) -> f64 {
    units
        .iter()
        .map(|(k, n)| k.atk() * (*n as f64))
        .sum::<f64>()
        .max(0.0)
}

pub fn defense_power(units: &Units, muralha: i32, anta: i32) -> f64 {
    let raw: f64 = units.iter().map(|(k, n)| k.def() * (*n as f64)).sum();
    raw * (1.0 + 0.05 * muralha.max(0) as f64) * (1.0 + 0.03 * anta.max(0) as f64)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Battle {
    pub attacker_survives: Units,
    pub defender_survives: Units,
    pub attacker_won: bool,
}

pub fn resolve_battle(attacker: &Units, defender: &Units, muralha: i32, anta: i32) -> Battle {
    let ap = attack_power(attacker);
    let dp = defense_power(defender, muralha, anta);
    if ap <= 0.0 && dp <= 0.0 {
        return Battle {
            attacker_survives: Units::new(),
            defender_survives: Units::new(),
            attacker_won: false,
        };
    }
    if ap <= 0.0 {
        return Battle {
            attacker_survives: Units::new(),
            defender_survives: defender.clone(),
            attacker_won: false,
        };
    }
    if dp <= 0.0 {
        return Battle {
            attacker_survives: attacker.clone(),
            defender_survives: Units::new(),
            attacker_won: true,
        };
    }
    if ap > dp {
        let keep = (1.0 - (dp / ap) * 0.55).clamp(0.05, 1.0);
        Battle {
            attacker_survives: scale_units(attacker, keep),
            defender_survives: Units::new(),
            attacker_won: true,
        }
    } else {
        let keep = (1.0 - (ap / dp) * 0.55).clamp(0.05, 1.0);
        Battle {
            attacker_survives: Units::new(),
            defender_survives: scale_units(defender, keep),
            attacker_won: false,
        }
    }
}

pub fn is_scout(units: &Units) -> bool {
    let falcao = units.get(&Unit::Falcao).copied().unwrap_or(0);
    falcao > 0 && units_total(units) == falcao
}

pub fn has_serpe(units: &Units) -> bool {
    units.get(&Unit::Serpe).copied().unwrap_or(0) > 0
}

pub fn starter_resources() -> Resources {
    Resources {
        cobre: 240.0,
        estanho: 240.0,
        seara: 320.0,
        orvalho: 160.0,
    }
}

pub fn encanto_cost() -> f64 {
    80.0
}

pub fn encanto_secs() -> i64 {
    15 * 60
}

pub fn bind_offering() -> f64 {
    100.0
}

/// Public rules the native client shows (costs, names). Not a simulation.
pub fn catalog() -> serde_json::Value {
    let mut buildings = Vec::new();
    for b in Building::all() {
        let mut levels = Vec::new();
        for lv in 0..10 {
            let (cost, bronze) = upgrade_cost(*b, lv);
            levels.push(serde_json::json!({
                "from": lv,
                "cost": cost,
                "bronze": bronze,
                "secs": upgrade_secs(lv),
            }));
        }
        buildings.push(serde_json::json!({
            "id": b.as_str(),
            "title": b.title(),
        "levels": levels,
        }));
    }
    let mut units = Vec::new();
    for u in Unit::all() {
        let (cost, bronze) = u.train_cost();
        units.push(serde_json::json!({
            "id": u.as_str(),
            "title": u.title(),
            "cost": cost,
            "bronze": bronze,
            "secs": u.train_secs(),
            "atk": u.atk(),
            "def": u.def(),
            "speed": u.speed(),
        }));
    }
    serde_json::json!({
        "buildings": buildings,
        "units": units,
        "encanto_cost": encanto_cost(),
        "encanto_secs": encanto_secs(),
        "bind_offering": bind_offering(),
        "max_seats": MAX_SEATS,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bronze_is_the_min_pair() {
        let mut r = Resources {
            cobre: 10.0,
            estanho: 4.0,
            seara: 0.0,
            orvalho: 0.0,
        };
        assert_eq!(r.bronze_available(), 4.0);
        assert!(r.spend_bronze(3.0));
        assert!((r.cobre - 7.0).abs() < 1e-9);
        assert!((r.estanho - 1.0).abs() < 1e-9);
        assert!(!r.spend_bronze(2.0));
    }

    #[test]
    fn production_grows_with_level() {
        assert!(prod_per_hour(Building::Mina, 2) > prod_per_hour(Building::Mina, 1));
        assert_eq!(prod_per_hour(Building::Mina, 0), 0.0);
        let hour = accrue(Building::Eira, 1, 3600);
        assert!((hour - 40.0).abs() < 1e-6);
    }

    #[test]
    fn winner_keeps_some_loser_keeps_none() {
        let mut atk = Units::new();
        atk.insert(Unit::Guerreiro, 10);
        let mut def = Units::new();
        def.insert(Unit::Pastor, 2);
        let b = resolve_battle(&atk, &def, 0, 0);
        assert!(b.attacker_won);
        assert_eq!(units_total(&b.defender_survives), 0);
        assert!(units_total(&b.attacker_survives) > 0);
    }

    #[test]
    fn scout_is_only_falcao() {
        let mut u = Units::new();
        u.insert(Unit::Falcao, 3);
        assert!(is_scout(&u));
        u.insert(Unit::Pastor, 1);
        assert!(!is_scout(&u));
    }

    #[test]
    fn travel_uses_slowest() {
        let mut u = Units::new();
        u.insert(Unit::Falcao, 1);
        u.insert(Unit::Javali, 1);
        let falcao_only = {
            let mut f = Units::new();
            f.insert(Unit::Falcao, 1);
            f
        };
        assert!(travel_secs(3, &u) > travel_secs(3, &falcao_only));
    }

    #[test]
    fn five_seat_cap_constant() {
        assert_eq!(MAX_SEATS, 5);
    }
}
