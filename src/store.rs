//! SQLite world. Tick, seats, armies, reports.

use crate::hex::{disk, seats_on_ring, Hex};
use crate::sim::{
    accrue, add_units, bind_offering, encanto_cost, encanto_secs, has_serpe, is_scout,
    resolve_battle, starter_resources, storage_cap, sub_units, travel_secs,
    units_from_json, units_to_json, units_total, upgrade_cost, upgrade_secs, Building, Resources,
    Unit, Units, MAP_RADIUS, MAX_SEATS, SEAT_RING,
};
use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

pub struct Store {
    conn: Mutex<Connection>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SeatInfo {
    pub idx: i32,
    pub q: i32,
    pub r: i32,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub seats_taken: usize,
    pub seats_max: usize,
    pub clock: i64,
    pub clock_local: String,
    pub seats: Vec<SeatInfo>,
}

fn now_unix() -> i64 {
    chrono::Local::now().timestamp()
}

fn hash_pass(pass: &str, salt: &str) -> String {
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update(b":");
    h.update(pass.as_bytes());
    hex::encode(h.finalize())
}

fn random_hex(n: usize) -> String {
    use rand::RngCore;
    let mut buf = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut buf);
    hex::encode(buf)
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path).with_context(|| format!("open {}", path.display()))?;
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                salt TEXT NOT NULL,
                pass_hash TEXT NOT NULL,
                created INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS invites (
                code TEXT PRIMARY KEY,
                created INTEGER NOT NULL,
                used_by INTEGER REFERENCES accounts(id)
            );
            CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                account_id INTEGER NOT NULL REFERENCES accounts(id),
                created INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tiles (
                q INTEGER NOT NULL,
                r INTEGER NOT NULL,
                kind TEXT NOT NULL,
                PRIMARY KEY (q, r)
            );
            CREATE TABLE IF NOT EXISTS seats (
                idx INTEGER PRIMARY KEY,
                q INTEGER NOT NULL,
                r INTEGER NOT NULL,
                account_id INTEGER UNIQUE REFERENCES accounts(id)
            );
            CREATE TABLE IF NOT EXISTS castros (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                q INTEGER NOT NULL,
                r INTEGER NOT NULL,
                owner_id INTEGER REFERENCES accounts(id),
                name TEXT NOT NULL,
                cobre REAL NOT NULL,
                estanho REAL NOT NULL,
                seara REAL NOT NULL,
                orvalho REAL NOT NULL,
                encanto_until INTEGER NOT NULL DEFAULT 0,
                last_accrue INTEGER NOT NULL,
                train_kind TEXT,
                train_count INTEGER NOT NULL DEFAULT 0,
                train_done INTEGER,
                UNIQUE(q,r)
            );
            CREATE TABLE IF NOT EXISTS buildings (
                castro_id INTEGER NOT NULL REFERENCES castros(id),
                kind TEXT NOT NULL,
                level INTEGER NOT NULL,
                upgrade_done INTEGER,
                PRIMARY KEY (castro_id, kind)
            );
            CREATE TABLE IF NOT EXISTS garrison (
                castro_id INTEGER NOT NULL REFERENCES castros(id),
                kind TEXT NOT NULL,
                count INTEGER NOT NULL,
                PRIMARY KEY (castro_id, kind)
            );
            CREATE TABLE IF NOT EXISTS armies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                owner_id INTEGER NOT NULL REFERENCES accounts(id),
                from_q INTEGER NOT NULL,
                from_r INTEGER NOT NULL,
                to_q INTEGER NOT NULL,
                to_r INTEGER NOT NULL,
                depart INTEGER NOT NULL,
                arrive INTEGER NOT NULL,
                mission TEXT NOT NULL,
                units TEXT NOT NULL,
                offering REAL NOT NULL DEFAULT 0,
                coming_home INTEGER NOT NULL DEFAULT 0,
                loot TEXT
            );
            CREATE TABLE IF NOT EXISTS reports (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id INTEGER NOT NULL REFERENCES accounts(id),
                created INTEGER NOT NULL,
                title TEXT NOT NULL,
                body TEXT NOT NULL
            );
            "#,
        )?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.ensure_world()?;
        Ok(store)
    }

    fn ensure_world(&self) -> Result<()> {
        let conn = self.conn.lock().expect("db");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM tiles", [], |r| r.get(0))?;
        if n > 0 {
            return Ok(());
        }
        let now = now_unix();
        conn.execute(
            "INSERT INTO meta(k,v) VALUES ('created', ?1), ('last_tick', ?1)",
            params![now.to_string()],
        )?;
        let seats = seats_on_ring(SEAT_RING);
        let seat_set: std::collections::HashSet<(i32, i32)> =
            seats.iter().map(|h| (h.q, h.r)).collect();
        for (i, h) in disk(MAP_RADIUS).into_iter().enumerate() {
            let kind = if seat_set.contains(&(h.q, h.r)) {
                "seat"
            } else if (h.q + h.r * 3 + i as i32).rem_euclid(7) == 0 && h.distance(Hex::new(0, 0)) > 1
            {
                "mamoa"
            } else {
                "outeiro"
            };
            conn.execute(
                "INSERT INTO tiles(q,r,kind) VALUES (?1,?2,?3)",
                params![h.q, h.r, kind],
            )?;
            if kind == "mamoa" {
                let _ = Self::insert_castro_locked(
                    &conn,
                    h,
                    None,
                    &format!("Mamoa {},{}", h.q, h.r),
                    Resources {
                        cobre: 80.0,
                        estanho: 80.0,
                        seara: 80.0,
                        orvalho: 40.0,
                    },
                    now,
                    true,
                )?;
            }
        }
        for (idx, h) in seats.iter().enumerate() {
            conn.execute(
                "INSERT INTO seats(idx,q,r) VALUES (?1,?2,?3)",
                params![idx as i32, h.q, h.r],
            )?;
        }
        Ok(())
    }

    fn insert_castro_locked(
        conn: &Connection,
        h: Hex,
        owner: Option<i64>,
        name: &str,
        res: Resources,
        now: i64,
        mamoa: bool,
    ) -> Result<i64> {
        conn.execute(
            "INSERT INTO castros(q,r,owner_id,name,cobre,estanho,seara,orvalho,last_accrue)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                h.q,
                h.r,
                owner,
                name,
                res.cobre,
                res.estanho,
                res.seara,
                res.orvalho,
                now
            ],
        )?;
        let id = conn.last_insert_rowid();
        let casa = if mamoa { 1 } else { 1 };
        for b in Building::all() {
            let level = match b {
                Building::Casa => casa,
                Building::Eira if !mamoa => 1,
                _ => 0,
            };
            conn.execute(
                "INSERT INTO buildings(castro_id,kind,level) VALUES (?1,?2,?3)",
                params![id, b.as_str(), level],
            )?;
        }
        let pastors = if mamoa { 12 } else { 20 };
        conn.execute(
            "INSERT INTO garrison(castro_id,kind,count) VALUES (?1,'pastor',?2)",
            params![id, pastors],
        )?;
        Ok(id)
    }

    pub fn invite(&self) -> Result<String> {
        let conn = self.conn.lock().expect("db");
        let taken: i64 = conn.query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))?;
        if taken as usize >= MAX_SEATS {
            bail!("mourama: all five seats are taken.");
        }
        let code = format!("moura-{}", &random_hex(4)[..6]);
        conn.execute(
            "INSERT INTO invites(code,created) VALUES (?1,?2)",
            params![code, now_unix()],
        )?;
        Ok(code)
    }

    pub fn seats_taken(&self) -> Result<usize> {
        let conn = self.conn.lock().expect("db");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    pub fn status(&self) -> Result<Status> {
        self.tick(now_unix())?;
        let conn = self.conn.lock().expect("db");
        let taken: i64 = conn.query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))?;
        let clock = now_unix();
        let clock_local = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string();
        let mut seats = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT s.idx, s.q, s.r, a.name FROM seats s LEFT JOIN accounts a ON a.id = s.account_id ORDER BY s.idx",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(SeatInfo {
                idx: r.get(0)?,
                q: r.get(1)?,
                r: r.get(2)?,
                name: r.get(3)?,
            })
        })?;
        for row in rows {
            seats.push(row?);
        }
        Ok(Status {
            seats_taken: taken as usize,
            seats_max: MAX_SEATS,
            clock,
            clock_local,
            seats,
        })
    }

    pub fn claim(&self, invite: &str, name: &str, pass: &str) -> Result<String> {
        let name = name.trim();
        if name.len() < 2 || name.len() > 24 {
            bail!("mourama: a court name is 2–24 letters.");
        }
        if pass.len() < 4 {
            bail!("mourama: pick a longer word to keep the seat.");
        }
        let now = now_unix();
        self.tick(now)?;
        let conn = self.conn.lock().expect("db");
        let taken: i64 = conn.query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))?;
        if taken as usize >= MAX_SEATS {
            bail!("mourama: all five seats are taken.");
        }
        let invite_row: Option<(String, Option<i64>)> = conn
            .query_row(
                "SELECT code, used_by FROM invites WHERE code = ?1",
                params![invite.trim()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((_, used_by)) = invite_row else {
            bail!("mourama: that invite does not open a seat.");
        };
        if used_by.is_some() {
            bail!("mourama: that invite already seated someone.");
        }
        let salt = random_hex(8);
        let pass_hash = hash_pass(pass, &salt);
        conn.execute(
            "INSERT INTO accounts(name,salt,pass_hash,created) VALUES (?1,?2,?3,?4)",
            params![name, salt, pass_hash, now],
        )?;
        let account_id = conn.last_insert_rowid();
        conn.execute(
            "UPDATE invites SET used_by = ?1 WHERE code = ?2",
            params![account_id, invite.trim()],
        )?;
        let seat: (i32, i32, i32) = conn.query_row(
            "SELECT idx, q, r FROM seats WHERE account_id IS NULL ORDER BY idx LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        conn.execute(
            "UPDATE seats SET account_id = ?1 WHERE idx = ?2",
            params![account_id, seat.0],
        )?;
        conn.execute(
            "UPDATE tiles SET kind = 'castro' WHERE q = ?1 AND r = ?2",
            params![seat.1, seat.2],
        )?;
        let _ = Self::insert_castro_locked(
            &conn,
            Hex::new(seat.1, seat.2),
            Some(account_id),
            name,
            starter_resources(),
            now,
            false,
        )?;
        let token = random_hex(16);
        conn.execute(
            "INSERT INTO sessions(token, account_id, created) VALUES (?1,?2,?3)",
            params![token, account_id, now],
        )?;
        Ok(token)
    }

    pub fn login(&self, name: &str, pass: &str) -> Result<String> {
        let conn = self.conn.lock().expect("db");
        let row: Option<(i64, String, String)> = conn
            .query_row(
                "SELECT id, salt, pass_hash FROM accounts WHERE name = ?1",
                params![name.trim()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((id, salt, hash)) = row else {
            bail!("mourama: no court by that name.");
        };
        if hash_pass(pass, &salt) != hash {
            bail!("mourama: that word does not open this court.");
        }
        let token = random_hex(16);
        conn.execute(
            "INSERT INTO sessions(token, account_id, created) VALUES (?1,?2,?3)",
            params![token, id, now_unix()],
        )?;
        Ok(token)
    }

    pub fn account_for(&self, token: &str) -> Result<i64> {
        let conn = self.conn.lock().expect("db");
        let id: Option<i64> = conn
            .query_row(
                "SELECT account_id FROM sessions WHERE token = ?1",
                params![token],
                |r| r.get(0),
            )
            .optional()?;
        id.ok_or_else(|| anyhow::anyhow!("mourama: sit down again — that session went cold."))
    }

    pub fn tick(&self, now: i64) -> Result<()> {
        let conn = self.conn.lock().expect("db");
        Self::tick_locked(&conn, now)
    }

    fn tick_locked(conn: &Connection, now: i64) -> Result<()> {
        let ids: Vec<i64> = {
            let mut stmt = conn.prepare("SELECT id FROM castros")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            let mut v = Vec::new();
            for r in rows {
                v.push(r?);
            }
            v
        };
        for id in ids {
            Self::accrue_castro(conn, id, now)?;
            Self::finish_upgrades(conn, id, now)?;
            Self::finish_training(conn, id, now)?;
        }
        let army_ids: Vec<i64> = {
            let mut stmt = conn.prepare("SELECT id FROM armies WHERE arrive <= ?1")?;
            let rows = stmt.query_map(params![now], |r| r.get(0))?;
            let mut v = Vec::new();
            for r in rows {
                v.push(r?);
            }
            v
        };
        for id in army_ids {
            Self::resolve_army(conn, id, now)?;
        }
        conn.execute(
            "INSERT INTO meta(k,v) VALUES('last_tick', ?1)
             ON CONFLICT(k) DO UPDATE SET v = excluded.v",
            params![now.to_string()],
        )?;
        Ok(())
    }

    fn levels(conn: &Connection, castro_id: i64) -> Result<BTreeMap<Building, (i32, Option<i64>)>> {
        let mut stmt =
            conn.prepare("SELECT kind, level, upgrade_done FROM buildings WHERE castro_id = ?1")?;
        let rows = stmt.query_map(params![castro_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?))
        })?;
        let mut m = BTreeMap::new();
        for row in rows {
            let (k, level, done) = row?;
            if let Some(b) = Building::parse(&k) {
                m.insert(b, (level, done));
            }
        }
        Ok(m)
    }

    fn garrison(conn: &Connection, castro_id: i64) -> Result<Units> {
        let mut stmt = conn.prepare("SELECT kind, count FROM garrison WHERE castro_id = ?1")?;
        let rows = stmt.query_map(params![castro_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        let mut u = Units::new();
        for row in rows {
            let (k, n) = row?;
            if let Some(unit) = Unit::parse(&k) {
                if n > 0 {
                    u.insert(unit, n);
                }
            }
        }
        Ok(u)
    }

    fn set_garrison(conn: &Connection, castro_id: i64, units: &Units) -> Result<()> {
        conn.execute(
            "DELETE FROM garrison WHERE castro_id = ?1",
            params![castro_id],
        )?;
        for (k, n) in units {
            if *n > 0 {
                conn.execute(
                    "INSERT INTO garrison(castro_id,kind,count) VALUES (?1,?2,?3)",
                    params![castro_id, k.as_str(), n],
                )?;
            }
        }
        Ok(())
    }

    fn write_resources(conn: &Connection, id: i64, res: &Resources, last: i64) -> Result<()> {
        conn.execute(
            "UPDATE castros SET cobre=?1, estanho=?2, seara=?3, orvalho=?4, last_accrue=?5 WHERE id=?6",
            params![res.cobre, res.estanho, res.seara, res.orvalho, last, id],
        )?;
        Ok(())
    }

    fn accrue_castro(conn: &Connection, id: i64, now: i64) -> Result<()> {
        let (mut res, _enc, last, _name, _owner): (Resources, i64, i64, String, Option<i64>) = conn
            .query_row(
                "SELECT cobre,estanho,seara,orvalho,encanto_until,last_accrue,name,owner_id FROM castros WHERE id=?1",
                params![id],
                |r| {
                    Ok((
                        Resources {
                            cobre: r.get(0)?,
                            estanho: r.get(1)?,
                            seara: r.get(2)?,
                            orvalho: r.get(3)?,
                        },
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                    ))
                },
            )?;
        let dt = now - last;
        if dt <= 0 {
            return Ok(());
        }
        let levels = Self::levels(conn, id)?;
        let celeiro = levels.get(&Building::Celeiro).map(|x| x.0).unwrap_or(0);
        res.seara += accrue(
            Building::Eira,
            levels.get(&Building::Eira).map(|x| x.0).unwrap_or(0),
            dt,
        );
        res.cobre += accrue(
            Building::Mina,
            levels.get(&Building::Mina).map(|x| x.0).unwrap_or(0),
            dt,
        );
        res.estanho += accrue(
            Building::Veio,
            levels.get(&Building::Veio).map(|x| x.0).unwrap_or(0),
            dt,
        );
        res.orvalho += accrue(
            Building::Fonte,
            levels.get(&Building::Fonte).map(|x| x.0).unwrap_or(0),
            dt,
        );
        res.clamp_cap(storage_cap(celeiro));
        Self::write_resources(conn, id, &res, now)?;
        Ok(())
    }

    fn finish_upgrades(conn: &Connection, id: i64, now: i64) -> Result<()> {
        conn.execute(
            "UPDATE buildings SET level = level + 1, upgrade_done = NULL
             WHERE castro_id = ?1 AND upgrade_done IS NOT NULL AND upgrade_done <= ?2",
            params![id, now],
        )?;
        Ok(())
    }

    fn finish_training(conn: &Connection, id: i64, now: i64) -> Result<()> {
        let row: Option<(Option<String>, i64, Option<i64>)> = conn
            .query_row(
                "SELECT train_kind, train_count, train_done FROM castros WHERE id=?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((Some(kind), count, Some(done))) = row else {
            return Ok(());
        };
        if done > now || count <= 0 {
            return Ok(());
        }
        if let Some(unit) = Unit::parse(&kind) {
            let mut g = Self::garrison(conn, id)?;
            *g.entry(unit).or_insert(0) += count;
            Self::set_garrison(conn, id, &g)?;
        }
        conn.execute(
            "UPDATE castros SET train_kind=NULL, train_count=0, train_done=NULL WHERE id=?1",
            params![id],
        )?;
        Ok(())
    }

    fn report(conn: &Connection, account_id: i64, now: i64, title: &str, body: serde_json::Value) -> Result<()> {
        conn.execute(
            "INSERT INTO reports(account_id,created,title,body) VALUES (?1,?2,?3,?4)",
            params![account_id, now, title, body.to_string()],
        )?;
        Ok(())
    }

    fn resolve_army(conn: &Connection, army_id: i64, now: i64) -> Result<()> {
        let (owner, from_q, from_r, to_q, to_r, mission, units_json, offering, returning, loot_json): (
            i64,
            i32,
            i32,
            i32,
            i32,
            String,
            String,
            f64,
            i64,
            Option<String>,
        ) = conn.query_row(
            "SELECT owner_id,from_q,from_r,to_q,to_r,mission,units,offering,coming_home,loot FROM armies WHERE id=?1",
            params![army_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                ))
            },
        )?;
        let units = units_from_json(&serde_json::from_str(&units_json).unwrap_or(serde_json::json!({})));
        conn.execute("DELETE FROM armies WHERE id=?1", params![army_id])?;

        if returning != 0 {
            if let Some(home) = Self::castro_at(conn, to_q, to_r)? {
                let mut g = Self::garrison(conn, home)?;
                add_units(&mut g, &units);
                Self::set_garrison(conn, home, &g)?;
                if let Some(loot_s) = loot_json {
                    if let Ok(v) = serde_json::from_str::<Resources>(&loot_s) {
                        Self::accrue_castro(conn, home, now)?;
                        let (mut res, ..): (Resources, i64, i64, String, Option<i64>) = conn.query_row(
                            "SELECT cobre,estanho,seara,orvalho,encanto_until,last_accrue,name,owner_id FROM castros WHERE id=?1",
                            params![home],
                            |r| {
                                Ok((
                                    Resources {
                                        cobre: r.get(0)?,
                                        estanho: r.get(1)?,
                                        seara: r.get(2)?,
                                        orvalho: r.get(3)?,
                                    },
                                    r.get(4)?,
                                    r.get(5)?,
                                    r.get(6)?,
                                    r.get(7)?,
                                ))
                            },
                        )?;
                        res.add(v);
                        let levels = Self::levels(conn, home)?;
                        let celeiro = levels.get(&Building::Celeiro).map(|x| x.0).unwrap_or(0);
                        res.clamp_cap(storage_cap(celeiro));
                        Self::write_resources(conn, home, &res, now)?;
                    }
                }
                Self::report(
                    conn,
                    owner,
                    now,
                    "Returned",
                    serde_json::json!({ "to": [to_q, to_r], "units": units_to_json(&units) }),
                )?;
            }
            return Ok(());
        }

        let dest = Self::castro_at(conn, to_q, to_r)?;
        if mission == "scout" || is_scout(&units) {
            let body = if let Some(cid) = dest {
                Self::accrue_castro(conn, cid, now)?;
                let snap = Self::castro_snapshot(conn, cid, true)?;
                serde_json::json!({ "target": [to_q, to_r], "seen": snap })
            } else {
                serde_json::json!({ "target": [to_q, to_r], "seen": "empty outeiro" })
            };
            Self::report(conn, owner, now, "Falcão over the hill", body)?;
            Self::launch_return(conn, owner, to_q, to_r, from_q, from_r, &units, None, now)?;
            return Ok(());
        }

        if dest.is_none() {
            Self::report(
                conn,
                owner,
                now,
                "Empty outeiro",
                serde_json::json!({ "target": [to_q, to_r] }),
            )?;
            Self::launch_return(conn, owner, to_q, to_r, from_q, from_r, &units, None, now)?;
            return Ok(());
        }
        let cid = dest.unwrap();
        Self::accrue_castro(conn, cid, now)?;
        let (def_owner, def_name): (Option<i64>, String) = conn.query_row(
            "SELECT owner_id, name FROM castros WHERE id=?1",
            params![cid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if def_owner == Some(owner) {
            let mut g = Self::garrison(conn, cid)?;
            add_units(&mut g, &units);
            Self::set_garrison(conn, cid, &g)?;
            return Ok(());
        }
        let levels = Self::levels(conn, cid)?;
        let muralha = levels.get(&Building::Muralha).map(|x| x.0).unwrap_or(0);
        let anta = levels.get(&Building::Anta).map(|x| x.0).unwrap_or(0);
        let defenders = Self::garrison(conn, cid)?;
        let battle = resolve_battle(&units, &defenders, muralha, anta);
        Self::set_garrison(conn, cid, &battle.defender_survives)?;

        let mut loot = Resources::default();
        if battle.attacker_won {
            let (mut res, encanto, last, _, _): (Resources, i64, i64, String, Option<i64>) = conn.query_row(
                "SELECT cobre,estanho,seara,orvalho,encanto_until,last_accrue,name,owner_id FROM castros WHERE id=?1",
                params![cid],
                |r| {
                    Ok((
                        Resources {
                            cobre: r.get(0)?,
                            estanho: r.get(1)?,
                            seara: r.get(2)?,
                            orvalho: r.get(3)?,
                        },
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                    ))
                },
            )?;
            loot = res.scale(0.10);
            let _ = res.saturating_sub(loot);
            Self::write_resources(conn, cid, &res, last)?;
            let _ = encanto;

            let bind_ok = mission == "bind" && has_serpe(&battle.attacker_survives) && offering + 1e-9 >= bind_offering();
            if bind_ok {
                conn.execute(
                    "UPDATE castros SET owner_id=?1 WHERE id=?2",
                    params![owner, cid],
                )?;
                conn.execute(
                    "UPDATE tiles SET kind='castro' WHERE q=?1 AND r=?2",
                    params![to_q, to_r],
                )?;
                let mut g = Self::garrison(conn, cid)?;
                add_units(&mut g, &battle.attacker_survives);
                Self::set_garrison(conn, cid, &g)?;
                Self::report(
                    conn,
                    owner,
                    now,
                    &format!("Bound {def_name}"),
                    serde_json::json!({
                        "target": [to_q, to_r],
                        "survived": units_to_json(&battle.attacker_survives),
                    }),
                )?;
                if let Some(did) = def_owner {
                    Self::report(
                        conn,
                        did,
                        now,
                        &format!("{def_name} was bound"),
                        serde_json::json!({ "target": [to_q, to_r] }),
                    )?;
                }
                return Ok(());
            }
        }

        let title = if battle.attacker_won {
            format!("Raid on {def_name}")
        } else {
            format!("Turned back at {def_name}")
        };
        Self::report(
            conn,
            owner,
            now,
            &title,
            serde_json::json!({
                "target": [to_q, to_r],
                "won": battle.attacker_won,
                "survived": units_to_json(&battle.attacker_survives),
                "loot": loot,
            }),
        )?;
        if let Some(did) = def_owner {
            Self::report(
                conn,
                did,
                now,
                if battle.attacker_won {
                    format!("Raided at {def_name}")
                } else {
                    format!("Held {def_name}")
                }
                .as_str(),
                serde_json::json!({ "won_defense": !battle.attacker_won }),
            )?;
        }
        if units_total(&battle.attacker_survives) > 0 {
            let loot_s = if battle.attacker_won {
                Some(serde_json::to_string(&loot).unwrap_or_default())
            } else {
                None
            };
            Self::launch_return(
                conn,
                owner,
                to_q,
                to_r,
                from_q,
                from_r,
                &battle.attacker_survives,
                loot_s.as_deref(),
                now,
            )?;
        }
        Ok(())
    }

    fn launch_return(
        conn: &Connection,
        owner: i64,
        from_q: i32,
        from_r: i32,
        to_q: i32,
        to_r: i32,
        units: &Units,
        loot: Option<&str>,
        now: i64,
    ) -> Result<()> {
        let dist = Hex::new(from_q, from_r).distance(Hex::new(to_q, to_r));
        let secs = travel_secs(dist, units);
        conn.execute(
            "INSERT INTO armies(owner_id,from_q,from_r,to_q,to_r,depart,arrive,mission,units,coming_home,loot)
             VALUES (?1,?2,?3,?4,?5,?6,?7,'return',?8,1,?9)",
            params![
                owner,
                from_q,
                from_r,
                to_q,
                to_r,
                now,
                now + secs,
                units_to_json(units).to_string(),
                loot
            ],
        )?;
        Ok(())
    }

    fn castro_at(conn: &Connection, q: i32, r: i32) -> Result<Option<i64>> {
        conn.query_row(
            "SELECT id FROM castros WHERE q=?1 AND r=?2",
            params![q, r],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    fn castro_snapshot(conn: &Connection, id: i64, true_sight: bool) -> Result<serde_json::Value> {
        let (res, encanto, last, name, owner, q, r): (Resources, i64, i64, String, Option<i64>, i32, i32) =
            conn.query_row(
                "SELECT cobre,estanho,seara,orvalho,encanto_until,last_accrue,name,owner_id,q,r FROM castros WHERE id=?1",
                params![id],
                |row| {
                    Ok((
                        Resources {
                            cobre: row.get(0)?,
                            estanho: row.get(1)?,
                            seara: row.get(2)?,
                            orvalho: row.get(3)?,
                        },
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                    ))
                },
            )?;
        let _ = last;
        let levels = Self::levels(conn, id)?;
        let g = Self::garrison(conn, id)?;
        let veiled = !true_sight && encanto > now_unix();
        let mut bmap = serde_json::Map::new();
        for (k, (lvl, done)) in &levels {
            bmap.insert(
                k.as_str().to_string(),
                serde_json::json!({ "level": lvl, "upgrade_done": done, "title": k.title() }),
            );
        }
        let units = if veiled {
            serde_json::json!("encanto")
        } else {
            units_to_json(&g)
        };
        let res_v = if veiled {
            serde_json::json!("encanto")
        } else {
            serde_json::to_value(res)?
        };
        Ok(serde_json::json!({
            "id": id,
            "q": q,
            "r": r,
            "name": name,
            "owner_id": owner,
            "resources": res_v,
            "bronze": if veiled { serde_json::json!("encanto") } else { serde_json::json!(res.bronze_available()) },
            "buildings": bmap,
            "garrison": units,
            "encanto_until": encanto,
            "veiled": veiled,
        }))
    }

    pub fn me(&self, account_id: i64) -> Result<serde_json::Value> {
        let now = now_unix();
        self.tick(now)?;
        let conn = self.conn.lock().expect("db");
        let name: String = conn.query_row(
            "SELECT name FROM accounts WHERE id=?1",
            params![account_id],
            |r| r.get(0),
        )?;
        let mut stmt = conn.prepare("SELECT id FROM castros WHERE owner_id=?1")?;
        let ids: Vec<i64> = stmt
            .query_map(params![account_id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut hills = Vec::new();
        for id in ids {
            hills.push(Self::castro_snapshot(&conn, id, true)?);
        }
        let mut stmt = conn.prepare(
            "SELECT id,from_q,from_r,to_q,to_r,arrive,mission,units,coming_home FROM armies WHERE owner_id=?1 ORDER BY arrive",
        )?;
        let armies: Vec<serde_json::Value> = stmt
            .query_map(params![account_id], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "from": [r.get::<_, i32>(1)?, r.get::<_, i32>(2)?],
                    "to": [r.get::<_, i32>(3)?, r.get::<_, i32>(4)?],
                    "arrive": r.get::<_, i64>(5)?,
                    "mission": r.get::<_, String>(6)?,
                    "units": serde_json::from_str::<serde_json::Value>(&r.get::<_, String>(7)?).unwrap_or(serde_json::json!({})),
                    "returning": r.get::<_, i64>(8)? == 1,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(serde_json::json!({
            "account_id": account_id,
            "name": name,
            "hills": hills,
            "armies": armies,
            "clock": now,
        }))
    }

    pub fn map(&self, viewer: Option<i64>) -> Result<serde_json::Value> {
        let now = now_unix();
        self.tick(now)?;
        let conn = self.conn.lock().expect("db");
        let mut stmt = conn.prepare(
            "SELECT t.q, t.r, t.kind, c.id, c.owner_id, c.name, c.encanto_until
             FROM tiles t LEFT JOIN castros c ON c.q = t.q AND c.r = t.r",
        )?;
        let tiles: Vec<serde_json::Value> = stmt
            .query_map([], |r| {
                let q: i32 = r.get(0)?;
                let rr: i32 = r.get(1)?;
                let kind: String = r.get(2)?;
                let cid: Option<i64> = r.get(3)?;
                let owner: Option<i64> = r.get(4)?;
                let name: Option<String> = r.get(5)?;
                let encanto: Option<i64> = r.get(6)?;
                let mine = viewer.is_some() && owner == viewer;
                let veiled = !mine && encanto.unwrap_or(0) > now;
                Ok(serde_json::json!({
                    "q": q,
                    "r": rr,
                    "kind": kind,
                    "castro_id": cid,
                    "owner_id": owner,
                    "name": name,
                    "mine": mine,
                    "veiled": veiled,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut stmt = conn.prepare(
            "SELECT id, owner_id, from_q, from_r, to_q, to_r, arrive, mission, coming_home FROM armies",
        )?;
        let armies: Vec<serde_json::Value> = stmt
            .query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "owner_id": r.get::<_, i64>(1)?,
                    "from": [r.get::<_, i32>(2)?, r.get::<_, i32>(3)?],
                    "to": [r.get::<_, i32>(4)?, r.get::<_, i32>(5)?],
                    "arrive": r.get::<_, i64>(6)?,
                    "mission": r.get::<_, String>(7)?,
                    "returning": r.get::<_, i64>(8)? == 1,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(serde_json::json!({
            "radius": MAP_RADIUS,
            "clock": now,
            "tiles": tiles,
            "armies": armies,
        }))
    }

    pub fn reports(&self, account_id: i64) -> Result<serde_json::Value> {
        let conn = self.conn.lock().expect("db");
        let mut stmt = conn.prepare(
            "SELECT id, created, title, body FROM reports WHERE account_id=?1 ORDER BY id DESC LIMIT 40",
        )?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map(params![account_id], |r| {
                let body_s: String = r.get(3)?;
                let body = serde_json::from_str(&body_s).unwrap_or(serde_json::json!(body_s));
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "created": r.get::<_, i64>(1)?,
                    "title": r.get::<_, String>(2)?,
                    "body": body,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(serde_json::json!({ "reports": rows }))
    }

    pub fn upgrade(&self, account_id: i64, castro_id: i64, kind: Building) -> Result<()> {
        let now = now_unix();
        self.tick(now)?;
        let conn = self.conn.lock().expect("db");
        let owner: Option<i64> = conn.query_row(
            "SELECT owner_id FROM castros WHERE id=?1",
            params![castro_id],
            |r| r.get(0),
        )?;
        if owner != Some(account_id) {
            bail!("mourama: that hill is not yours to raise.");
        }
        Self::accrue_castro(&conn, castro_id, now)?;
        let levels = Self::levels(&conn, castro_id)?;
        let (level, upgrading) = levels.get(&kind).copied().unwrap_or((0, None));
        if upgrading.is_some() {
            bail!("mourama: that building is already rising.");
        }
        let casa = levels.get(&Building::Casa).map(|x| x.0).unwrap_or(0);
        if kind != Building::Casa && level >= casa {
            bail!("mourama: the casa circular must stand taller first.");
        }
        if level >= 10 {
            bail!("mourama: that building can rise no further.");
        }
        let (cost, bronze) = upgrade_cost(kind, level);
        let (mut res, _, last, _, _): (Resources, i64, i64, String, Option<i64>) = conn.query_row(
            "SELECT cobre,estanho,seara,orvalho,encanto_until,last_accrue,name,owner_id FROM castros WHERE id=?1",
            params![castro_id],
            |r| {
                Ok((
                    Resources {
                        cobre: r.get(0)?,
                        estanho: r.get(1)?,
                        seara: r.get(2)?,
                        orvalho: r.get(3)?,
                    },
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ))
            },
        )?;
        if bronze > 0.0 && !res.spend_bronze(bronze) {
            bail!("mourama: not enough bronze (cobre+estanho together).");
        }
        if !res.saturating_sub(cost) {
            bail!("mourama: the piles are too small for that rise.");
        }
        Self::write_resources(&conn, castro_id, &res, last)?;
        let done = now + upgrade_secs(level);
        conn.execute(
            "UPDATE buildings SET upgrade_done=?1 WHERE castro_id=?2 AND kind=?3",
            params![done, castro_id, kind.as_str()],
        )?;
        Ok(())
    }

    pub fn train(&self, account_id: i64, castro_id: i64, kind: Unit, count: i64) -> Result<()> {
        if count <= 0 || count > 50 {
            bail!("mourama: train between 1 and 50 at a time.");
        }
        let now = now_unix();
        self.tick(now)?;
        let conn = self.conn.lock().expect("db");
        let (owner, train_kind, train_count): (Option<i64>, Option<String>, i64) = conn.query_row(
            "SELECT owner_id, train_kind, train_count FROM castros WHERE id=?1",
            params![castro_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if owner != Some(account_id) {
            bail!("mourama: that curral is not yours.");
        }
        if train_kind.is_some() && train_count > 0 {
            bail!("mourama: the curral is already raising folk.");
        }
        let levels = Self::levels(&conn, castro_id)?;
        if levels.get(&Building::Curral).map(|x| x.0).unwrap_or(0) < 1 && kind != Unit::Pastor {
            bail!("mourama: raise a curral before training anyone but pastors.");
        }
        Self::accrue_castro(&conn, castro_id, now)?;
        let (mut res, _, last, _, _): (Resources, i64, i64, String, Option<i64>) = conn.query_row(
            "SELECT cobre,estanho,seara,orvalho,encanto_until,last_accrue,name,owner_id FROM castros WHERE id=?1",
            params![castro_id],
            |r| {
                Ok((
                    Resources {
                        cobre: r.get(0)?,
                        estanho: r.get(1)?,
                        seara: r.get(2)?,
                        orvalho: r.get(3)?,
                    },
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ))
            },
        )?;
        let (unit_cost, bronze) = kind.train_cost();
        let total = unit_cost.scale(count as f64);
        let bronze_total = bronze * count as f64;
        if bronze_total > 0.0 && !res.spend_bronze(bronze_total) {
            bail!("mourama: not enough bronze (cobre+estanho together).");
        }
        if !res.saturating_sub(total) {
            bail!("mourama: the piles are too small to train that many.");
        }
        Self::write_resources(&conn, castro_id, &res, last)?;
        let curral = levels.get(&Building::Curral).map(|x| x.0).unwrap_or(0) as f64;
        let factor = 1.0 + 0.1 * curral;
        let done = now + ((kind.train_secs() as f64 * count as f64) / factor).ceil() as i64;
        conn.execute(
            "UPDATE castros SET train_kind=?1, train_count=?2, train_done=?3 WHERE id=?4",
            params![kind.as_str(), count, done, castro_id],
        )?;
        Ok(())
    }

    pub fn send(
        &self,
        account_id: i64,
        from_id: i64,
        to_q: i32,
        to_r: i32,
        units: Units,
        mission: &str,
    ) -> Result<i64> {
        if units_total(&units) <= 0 {
            bail!("mourama: send someone, not an empty mist.");
        }
        let mission = match mission {
            "raid" | "bind" | "scout" => mission,
            _ => "raid",
        };
        if mission == "scout" && !is_scout(&units) {
            bail!("mourama: a scout is falcão only.");
        }
        if mission == "bind" && !has_serpe(&units) {
            bail!("mourama: a bind needs a serpe.");
        }
        let now = now_unix();
        self.tick(now)?;
        let conn = self.conn.lock().expect("db");
        let (owner, fq, fr): (Option<i64>, i32, i32) = conn.query_row(
            "SELECT owner_id, q, r FROM castros WHERE id=?1",
            params![from_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if owner != Some(account_id) {
            bail!("mourama: those folk are not yours to send.");
        }
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tiles WHERE q=?1 AND r=?2",
            params![to_q, to_r],
            |r| r.get(0),
        )?;
        if exists == 0 {
            bail!("mourama: that outeiro is off the wood.");
        }
        let mut g = Self::garrison(&conn, from_id)?;
        if !sub_units(&mut g, &units) {
            bail!("mourama: you do not hold that many on this hill.");
        }
        let mut offering = 0.0;
        if mission == "bind" {
            Self::accrue_castro(&conn, from_id, now)?;
            let (mut res, _, last, _, _): (Resources, i64, i64, String, Option<i64>) = conn.query_row(
                "SELECT cobre,estanho,seara,orvalho,encanto_until,last_accrue,name,owner_id FROM castros WHERE id=?1",
                params![from_id],
                |r| {
                    Ok((
                        Resources {
                            cobre: r.get(0)?,
                            estanho: r.get(1)?,
                            seara: r.get(2)?,
                            orvalho: r.get(3)?,
                        },
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                    ))
                },
            )?;
            offering = bind_offering();
            if !res.saturating_sub(Resources {
                orvalho: offering,
                ..Resources::default()
            }) {
                bail!("mourama: a bind wants {offering} orvalho at the anta.");
            }
            Self::write_resources(&conn, from_id, &res, last)?;
        }
        Self::set_garrison(&conn, from_id, &g)?;
        let dist = Hex::new(fq, fr).distance(Hex::new(to_q, to_r));
        let secs = travel_secs(dist, &units);
        conn.execute(
            "INSERT INTO armies(owner_id,from_q,from_r,to_q,to_r,depart,arrive,mission,units,offering,coming_home)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,0)",
            params![
                account_id,
                fq,
                fr,
                to_q,
                to_r,
                now,
                now + secs,
                mission,
                units_to_json(&units).to_string(),
                offering
            ],
        )?;
        Ok(now + secs)
    }

    pub fn encanto(&self, account_id: i64, castro_id: i64) -> Result<i64> {
        let now = now_unix();
        self.tick(now)?;
        let conn = self.conn.lock().expect("db");
        let owner: Option<i64> = conn.query_row(
            "SELECT owner_id FROM castros WHERE id=?1",
            params![castro_id],
            |r| r.get(0),
        )?;
        if owner != Some(account_id) {
            bail!("mourama: you cannot veil a hill that is not yours.");
        }
        Self::accrue_castro(&conn, castro_id, now)?;
        let (mut res, _, last, _, _): (Resources, i64, i64, String, Option<i64>) = conn.query_row(
            "SELECT cobre,estanho,seara,orvalho,encanto_until,last_accrue,name,owner_id FROM castros WHERE id=?1",
            params![castro_id],
            |r| {
                Ok((
                    Resources {
                        cobre: r.get(0)?,
                        estanho: r.get(1)?,
                        seara: r.get(2)?,
                        orvalho: r.get(3)?,
                    },
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                ))
            },
        )?;
        let cost = encanto_cost();
        if !res.saturating_sub(Resources {
            orvalho: cost,
            ..Resources::default()
        }) {
            bail!("mourama: encanto wants {cost} orvalho.");
        }
        let until = now + encanto_secs();
        conn.execute(
            "UPDATE castros SET cobre=?1,estanho=?2,seara=?3,orvalho=?4,last_accrue=?5,encanto_until=?6 WHERE id=?7",
            params![res.cobre, res.estanho, res.seara, res.orvalho, last, until, castro_id],
        )?;
        Ok(until)
    }

    pub fn hill_at(&self, viewer: i64, q: i32, r: i32) -> Result<serde_json::Value> {
        let now = now_unix();
        self.tick(now)?;
        let conn = self.conn.lock().expect("db");
        let cid = Self::castro_at(&conn, q, r)?;
        let Some(cid) = cid else {
            return Ok(serde_json::json!({ "empty": true, "q": q, "r": r }));
        };
        let owner: Option<i64> = conn.query_row(
            "SELECT owner_id FROM castros WHERE id=?1",
            params![cid],
            |r| r.get(0),
        )?;
        let true_sight = owner == Some(viewer);
        Self::castro_snapshot(&conn, cid, true_sight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::MAX_SEATS;

    fn mem() -> Store {
        let dir = tempfile::tempdir().unwrap();
        Store::open(&dir.path().join("w.sqlite")).unwrap()
    }

    #[test]
    fn sixth_seat_is_refused() {
        let s = mem();
        for i in 0..MAX_SEATS {
            let code = s.invite().unwrap();
            s.claim(&code, &format!("court{i}"), "secret").unwrap();
        }
        assert!(s.invite().is_err());
        assert!(s.claim("moura-nope", "intruder", "secret").is_err());
        assert_eq!(s.seats_taken().unwrap(), 5);
    }

    #[test]
    fn upgrade_and_accrue() {
        let s = mem();
        let code = s.invite().unwrap();
        let token = s.claim(&code, "Gil", "secret").unwrap();
        let id = s.account_for(&token).unwrap();
        let me = s.me(id).unwrap();
        let cid = me["hills"][0]["id"].as_i64().unwrap();
        s.upgrade(id, cid, Building::Mina).unwrap();
        // fast-forward by ticking far
        let now = now_unix() + 10_000;
        s.tick(now).unwrap();
        let me = s.me(id).unwrap();
        assert!(me["hills"][0]["buildings"]["mina"]["level"].as_i64().unwrap() >= 1);
    }
}
