use std::{
    collections::HashSet,
    thread,
    time::{Duration, Instant},
};

use eyre::{bail, Context, ContextCompat, Result};
use flume::{bounded, Receiver, Sender};
use geo::{Closest, Distance, Haversine, HaversineClosestPoint, Intersects, Line, Point};
use regex::Regex;
use reqwest::get;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

#[derive(Debug, Clone, Deserialize)]
struct VatsimData {
    pilots: Vec<Pilot>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Pilot {
    pub callsign: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: i64,
    #[serde(rename = "groundspeed")]
    pub ground_speed: i64,
    pub flight_plan: Option<FlightPlan>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct FlightPlan {
    pub departure: String,
    pub arrival: String,
    pub route: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Waypoint {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
}

impl Waypoint {
    pub fn unknown(lat: f64, lon: f64) -> Waypoint {
        Waypoint {
            id: "unknown".to_owned(),
            lat,
            lon,
        }
    }

    pub fn new(id: String, lat: f64, lon: f64) -> Waypoint {
        Waypoint { id, lat, lon }
    }
}

pub struct Route {
    callsign: String,
    current_route: Vec<String>,
    previous_route: Vec<String>,
    last_vatsim_update: Instant,
    route_waypoints: Vec<Waypoint>,
    aircraft_waypoints: Vec<Waypoint>,
    last_waypoint_count: usize,
    last_stat: RouteStatistics,
    tx: Sender<RouteRequest>,
    rx: Receiver<Result<Vec<Waypoint>>>,
}

struct RouteRequest {
    route_tokens: Vec<String>,
    flight_plan: FlightPlan,
}

async fn get_vatsim_data(callsign: &str) -> Result<Pilot> {
    let response = get("https://data.vatsim.net/v3/vatsim-data.json").await?;
    if !response.status().is_success() {
        bail!("Failed to fetch vatsim data");
    }

    let vatsim_data: VatsimData = response.json().await?;
    let pilot = vatsim_data
        .pilots
        .iter()
        .position(|p| p.callsign == callsign);
    if pilot.is_none() {
        bail!("Pilot not yet connected to vatsim!");
    }
    Ok(vatsim_data.pilots[pilot.unwrap()].clone())
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RouteStatistics {
    pub leftover_route: Vec<String>,
    pub next_waypoint: String,
    pub prev_waypoint: String,
    pub route_deviation: f64,
    pub route_progress: f64,
    pub dist_next_wp: f64,
    pub in_loop: bool,
    pub stuck: bool,
    pub pilot: Pilot,
}

impl Route {
    pub fn new(nav_db: &str, callsign: &str) -> Result<Route> {
        let conn = Connection::open(nav_db).context("Could not open nav db")?;
        let (tx, rx) = bounded(1);
        let (tx_r, rx_r) = bounded(1);
        thread::spawn(move || InnerRoute::start(InnerRoute { conn }, tx_r, rx));

        Ok(Route {
            callsign: callsign.to_owned(),
            current_route: Vec::new(),
            previous_route: Vec::new(),
            route_waypoints: Vec::new(),
            aircraft_waypoints: Vec::new(),
            last_waypoint_count: 0,
            last_vatsim_update: Instant::now() - Duration::from_secs(16),
            last_stat: RouteStatistics::default(),
            tx,
            rx: rx_r,
        })
    }

    pub async fn route_statistics(&mut self) -> Result<RouteStatistics> {
        if self.last_vatsim_update.elapsed() < Duration::from_secs(15) {
            return Ok(self.last_stat.clone());
        }

        let pilot = get_vatsim_data(&self.callsign).await?;
        self.last_vatsim_update = Instant::now();

        let mut stuck = false;
        if self.aircraft_waypoints.len() == 120 {
            self.aircraft_waypoints.remove(0);
        }
        if let Some(last_wpt) = self.aircraft_waypoints.last() {
            if last_wpt.lat == pilot.latitude && last_wpt.lon == pilot.longitude {
                self.last_waypoint_count += 1;
            } else {
                self.last_waypoint_count = 0;
                self.aircraft_waypoints
                    .push(Waypoint::unknown(pilot.latitude, pilot.longitude));
            }

            stuck = self.last_waypoint_count > 10;
        } else {
            self.aircraft_waypoints
                .push(Waypoint::unknown(pilot.latitude, pilot.longitude));
        }

        self.previous_route = self.current_route.clone();
        let flight_plan = pilot
            .flight_plan
            .as_ref()
            .context("Pilot has no flight plan")?;
        self.current_route = flight_plan
            .route
            .split_whitespace()
            .filter(|s| s.to_uppercase() != "DCT")
            .map(|x| x.to_owned())
            .collect::<Vec<_>>();

        if self.current_route.len() != self.previous_route.len()
            || md5::compute(self.current_route.join(""))
                != md5::compute(self.previous_route.join(""))
        {
            self.tx
                .send_async(RouteRequest {
                    route_tokens: self.current_route.clone(),
                    flight_plan: flight_plan.clone(),
                })
                .await?;
            self.route_waypoints = self.rx.recv_async().await??;

            debug!("recomputing route waypoints");
            debug!(
                "new route: {:#?}",
                self.route_waypoints
                    .iter()
                    .map(|w| w.id.as_str())
                    .collect::<Vec<_>>()
                    .join(" -> ")
            );
        }

        if self.current_route.len() < 2 {
            bail!("Route is too short");
        }

        let in_loop = has_loop(&self.aircraft_waypoints);
        if in_loop {
            tokio::fs::write(
                "loops.json",
                serde_json::to_string_pretty(&self.aircraft_waypoints)?,
            )
            .await?;
        }

        let (prev_idx, _, prev, next, segment_deviation) =
            find_closest_segment(&self.route_waypoints, pilot.latitude, pilot.longitude)
                .context("Failed to find closest segment")?;

        let current_pos = Point::new(pilot.longitude, pilot.latitude);
        let next_pos = Point::new(next.lon, next.lat);

        let distance_to_next = Haversine.distance(current_pos, next_pos);
        let total_distance = route_length_nm(&self.route_waypoints);

        let mut done = route_length_nm(&self.route_waypoints[0..prev_idx]);
        done += Haversine.distance(Point::new(prev.lon, prev.lat), current_pos) / 1852.0;

        let mt_to_nmi = |m| m / 1852.0;
        let pct_complete = (done / total_distance) * 100.0;

        let leftover: Vec<String> = self
            .route_waypoints
            .iter()
            .skip(prev_idx)
            .map(|wpt| wpt.id.clone())
            .collect();

        self.last_stat = RouteStatistics {
            leftover_route: leftover,
            next_waypoint: next.id,
            prev_waypoint: prev.id,
            route_deviation: mt_to_nmi(segment_deviation),
            route_progress: pct_complete,
            dist_next_wp: mt_to_nmi(distance_to_next),
            in_loop,
            stuck,
            pilot,
        };

        Ok(self.last_stat.clone())
    }
}

struct InnerRoute {
    conn: Connection,
}

impl InnerRoute {
    fn start(self, tx: Sender<Result<Vec<Waypoint>>>, rx: Receiver<RouteRequest>) {
        while let Ok(RouteRequest {
            route_tokens,
            flight_plan,
        }) = rx.recv()
        {
            if let Err(err) = tx.send(self.get_waypoints(&route_tokens, &flight_plan)) {
                error!("Failed to send waypoints: {err}");
            }
        }
    }

    fn get_waypoints(
        &self,
        route_tokens: &[String],
        flight_plan: &FlightPlan,
    ) -> Result<Vec<Waypoint>> {
        let mut wps: Vec<Waypoint> = Vec::new();

        let first = route_tokens[0].clone();
        let sid_pts = self
            .fetch_procedure(flight_plan.departure.clone(), first.clone(), 'D')
            .unwrap_or_default();
        if !sid_pts.is_empty() {
            wps.extend(sid_pts);
        } else {
            self.expand_token(&mut wps, &first, "")?;
        }

        for i in 1..route_tokens.len() - 1 {
            self.expand_token(&mut wps, &route_tokens[i], &route_tokens[i + 1])?;
        }

        let last = route_tokens.last().unwrap();
        let star_pts = self
            .fetch_procedure(flight_plan.arrival.clone(), last.clone(), 'A')
            .unwrap_or_default();
        if !star_pts.is_empty() {
            wps.extend(star_pts);
        } else {
            self.expand_token(&mut wps, last, "")?;
        }

        if let Some(wpt) = self.get_airport(flight_plan.arrival.clone()) {
            wps.push(wpt);
        }

        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for item in wps {
            if seen.insert(item.id.clone()) {
                result.push(item);
            }
        }
        Ok(result)
    }

    fn expand_token(&self, wps: &mut Vec<Waypoint>, tok: &str, next_tok: &str) -> Result<()> {
        let base = tok.split('/').next().unwrap();
        let ll_re = Regex::new(r"^(\d{2})([NS])(\d{3})([EW])$").unwrap();
        if let Some(c) = ll_re.captures(base) {
            let dlat: f64 = c[1].parse().unwrap();
            let dlon: f64 = c[3].parse().unwrap();
            let lat = if &c[2] == "N" { dlat } else { -dlat };
            let lon = if &c[4] == "E" { dlon } else { -dlon };
            wps.push(Waypoint::new(base.to_string(), lat, lon));
            return Ok(());
        }

        if base.starts_with("NAT") && base.len() == 4 {
            if let Ok(pts) = self.fetch_nattrak(&base[3..4]) {
                for wpt in pts {
                    if !wpt.lat.is_nan() {
                        wps.push(wpt);
                    } else {
                        let fixes = self.get_fix(wpt.id)?;
                        wps.push(fixes[0].clone());
                    }
                }
            }
            return Ok(());
        }
        let fixes = self.get_fix(base.to_owned())?;
        if !fixes.is_empty() {
            if let Some(prev) = wps.last() {
                let mut min_dist = f64::INFINITY;
                let mut best = None;
                for cand in fixes {
                    let dist = Haversine.distance(
                        geo::Point::new(prev.lon, prev.lat),
                        geo::Point::new(cand.lon, cand.lat),
                    );
                    if dist < min_dist {
                        min_dist = dist;
                        best = Some(cand);
                    }
                }
                if let Some(best_fix) = best {
                    wps.push(best_fix);
                }
            } else {
                wps.push(fixes[0].clone());
            }
            return Ok(());
        }

        if !wps.is_empty() {
            let join_fix = wps.last().map(|wpt| wpt.id.clone()).unwrap();
            let exit_fix = next_tok.split('/').next().unwrap().to_string();
            let awy_pts = self.fetch_airway(base.to_owned(), join_fix, exit_fix)?;
            wps.extend(awy_pts);
        }
        Ok(())
    }

    fn fetch_airway(
        &self,
        awy: String,
        join_fix: String,
        exit_fix: String,
    ) -> Result<Vec<Waypoint>> {
        let mut out = Vec::new();
        let mut stmt = self.conn.prepare("SELECT waypoint_identifier, waypoint_latitude, waypoint_longitude FROM tbl_enroute_airways WHERE route_identifier = ? ORDER BY seqno DESC")?;
        let mut rows = stmt.query([awy.clone()])?;
        while let Ok(Some(r)) = rows.next() {
            out.push(Waypoint::new(r.get(0)?, r.get(1)?, r.get(2)?));
        }

        if out.is_empty() {
            return Ok(out);
        }

        let start = match out.iter().position(|wpt| wpt.id == join_fix) {
            Some(i) => i,
            None => return Ok(Vec::new()),
        };
        let end = match out.iter().position(|wpt| wpt.id == exit_fix) {
            Some(i) => i,
            None => return Ok(Vec::new()),
        };
        if start + 1 == end {
            return Ok(Vec::new());
        }
        if start + 1 > end {
            let mut res = out[end + 1..start].to_vec();
            res.reverse();
            return Ok(res);
        }
        Ok(out[(start + 1)..end].to_vec())
    }

    fn get_airport(&self, ident: String) -> Option<Waypoint> {
        let mut stmt = self.conn.prepare(
        "SELECT airport_ref_latitude, airport_ref_longitude FROM tbl_airports WHERE airport_identifier = ?"
    ).unwrap();
        stmt.query_row([ident.clone()], |r| Ok((r.get(0)?, r.get(1)?)))
            .map(|row| Waypoint::new(ident, row.0, row.1))
            .ok()
    }

    fn get_fix(&self, ident: String) -> Result<Vec<Waypoint>> {
        let mut candidates = Vec::new();

        let mut try_stmt = |sql: &str| -> rusqlite::Result<_> {
            let mut stmt = self.conn.prepare(sql)?;
            let mut rows = stmt.query([&ident])?;
            while let Ok(Some(row)) = rows.next() {
                candidates.push(Waypoint::new(ident.clone(), row.get(0)?, row.get(1)?));
            }
            Ok(())
        };

        try_stmt("SELECT waypoint_latitude, waypoint_longitude FROM tbl_enroute_waypoints WHERE waypoint_identifier = ?")?;
        try_stmt("SELECT waypoint_latitude, waypoint_longitude FROM tbl_terminal_waypoints WHERE waypoint_identifier = ?")?;
        try_stmt(
            "SELECT vor_latitude, vor_longitude FROM tbl_vhfnavaids WHERE vor_identifier = ?",
        )?;
        try_stmt(
            "SELECT ndb_latitude, ndb_longitude FROM tbl_enroute_ndbnavaids WHERE ndb_identifier = ?",
        )?;
        try_stmt(
            "SELECT ndb_latitude, ndb_longitude FROM tbl_terminal_ndbnavaids WHERE ndb_identifier = ?",
        )?;

        Ok(candidates)
    }

    fn fetch_procedure(
        &self,
        airport: String,
        proc_token: String,
        kind: char,
    ) -> Result<Vec<Waypoint>> {
        let table = match kind {
            'D' => "tbl_sids",
            'A' => "tbl_stars",
            _ => unreachable!(),
        };

        let raw = proc_token.split('/').next().unwrap();
        let re = Regex::new(r"^([A-Z]+?)(\d.*)?$").unwrap();
        let (wp_pref, num_suf) = re
            .captures(raw)
            .map(|c| {
                (
                    c.get(1).unwrap().as_str(),
                    c.get(2).map(|m| m.as_str()).unwrap_or(""),
                )
            })
            .unwrap_or((raw, ""));

        let sql = format!("SELECT DISTINCT procedure_identifier, transition_identifier FROM {table} WHERE airport_identifier = ?");
        let mut stmt = self.conn.prepare(&sql).unwrap();
        let mut rows = stmt.query([&airport]).unwrap();

        let mut best: Option<(String, Option<String>)> = None;
        let mut best_score = 0;

        while let Ok(Some(r)) = rows.next() {
            let proc_id: String = r.get(0)?;
            let trans_id: Option<String> = r.get(1)?;

            let full_key = match &trans_id {
                Some(t) => proc_id.clone() + t,
                None => proc_id.clone(),
            };

            let score = (raw.eq_ignore_ascii_case(&full_key) as usize) * 100
                + (proc_token.eq_ignore_ascii_case(&full_key) as usize) * 50
                + (wp_pref.eq_ignore_ascii_case(&full_key[..wp_pref.len()]) as usize) * 10
                + ((!num_suf.is_empty() && full_key.ends_with(num_suf)) as usize) * 5;

            if score > best_score {
                best_score = score;
                best = Some((proc_id.clone(), trans_id.clone()));
            }
        }

        let (proc_id, trans_id) = match best {
            Some(x) if best_score > 0 => x,
            _ => return Ok(Vec::new()),
        };
        let mut proc_rows = Vec::new();
        let sql = format!("SELECT waypoint_identifier, waypoint_latitude, waypoint_longitude FROM {table} WHERE airport_identifier = ? AND procedure_identifier = ? AND (transition_identifier = ? OR transition_identifier IS NULL) AND waypoint_latitude IS NOT NULL ORDER BY seqno {}", if kind == 'D' { "DESC" } else { "" });
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query([&airport, &proc_id, trans_id.as_deref().unwrap_or("")])?;
        while let Ok(Some(row)) = rows.next() {
            let id: String = row.get(0)?;
            let lat: f64 = row.get(1)?;
            let lon: f64 = row.get(2)?;
            proc_rows.push(Waypoint::new(id, lat, lon));
        }
        Ok(proc_rows)
    }

    fn fetch_nattrak(&self, track_id: &str) -> Result<Vec<Waypoint>, String> {
        let response = reqwest::blocking::get("https://nattrak.vatsim.net/api/tracks")
            .map_err(|e| e.to_string())?;

        #[derive(Debug, Clone, Deserialize)]
        struct NatTrack {
            identifier: String,
            active: bool,
            last_routeing: String,
        }

        let tracks: Vec<NatTrack> = response.json().map_err(|e| e.to_string())?;

        let track = tracks
            .iter()
            .find(|t| t.identifier.eq_ignore_ascii_case(track_id) && t.active)
            .ok_or_else(|| format!("Track {track_id} not found or not active"))?;

        let coord_re = Regex::new(r"^(\d{2}(?:\d{2})?)/(\d{2}(?:\d{2})?)$").unwrap();
        let mut pts = Vec::new();

        for tok in track.last_routeing.split_whitespace() {
            if let Some(caps) = coord_re.captures(tok) {
                let lat = parse_latlon_token(&caps[1]);
                let lon = parse_latlon_token(&caps[2]);
                pts.push(Waypoint::new(tok.to_string(), lat, -lon));
            }
        }
        Ok(pts)
    }
}

pub fn find_closest_segment(
    waypoints: &[Waypoint],
    lat: f64,
    lon: f64,
) -> Option<(usize, usize, Waypoint, Waypoint, f64)> {
    let p = Point::new(lon, lat);
    let mut best: Option<(usize, usize, Waypoint, Waypoint, f64)> = None;
    let mut best_dev_nm = f64::INFINITY;

    for i in 0..waypoints.len().saturating_sub(1) {
        let a = &waypoints[i];
        let b = &waypoints[i + 1];
        let pa = Point::new(a.lon, a.lat);
        let pb = Point::new(b.lon, b.lat);
        let line = Line::new(pa, pb);

        if let Closest::Intersection(proj) | Closest::SinglePoint(proj) =
            line.haversine_closest_point(&p)
        {
            let dev_m = Haversine.distance(p, proj);
            if dev_m < best_dev_nm {
                best_dev_nm = dev_m;
                best = Some((i, i + 1, a.clone(), b.clone(), dev_m));
            }
        }
    }

    best
}

fn route_length_nm(waypoints: &[Waypoint]) -> f64 {
    waypoints
        .windows(2)
        .map(|w| {
            let p1 = geo::Point::new(w[0].lon, w[0].lat);
            let p2 = geo::Point::new(w[1].lon, w[1].lat);
            Haversine.distance(p1, p2)
        })
        .sum::<f64>()
        / 1852.0
}

fn has_loop(wps: &[Waypoint]) -> bool {
    let pts: Vec<Point<f64>> = wps.iter().map(|wp| Point::new(wp.lon, wp.lat)).collect();

    for i in 0..pts.len().saturating_sub(1) {
        let a1 = pts[i];
        let a2 = pts[i + 1];
        let seg1 = Line::new(a1.0, a2.0);
        for j in (i + 2)..pts.len().saturating_sub(1) {
            if j == i + 1 {
                continue;
            }
            let b1 = pts[j];
            let b2 = pts[j + 1];
            let seg2 = Line::new(b1.0, b2.0);
            if seg1.intersects(&seg2) {
                return true;
            }
        }
    }
    false
}

fn parse_latlon_token(token: &str) -> f64 {
    match token.len() {
        2 => token.parse::<i32>().unwrap() as f64,
        4 => {
            let deg: f64 = token[0..2].parse().unwrap();
            let min: f64 = token[2..4].parse().unwrap();
            deg + min / 60.0
        }
        _ => f64::NAN,
    }
}
