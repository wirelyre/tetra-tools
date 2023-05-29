use std::collections::HashSet;

use miniserde::Deserialize;

#[allow(dead_code)]
pub struct Physics {
    name: String,
    color: u8,
    minoes: [Vec<(u8, u8)>; 4],
    width: u8,
    height: u8,
    rotations: Rotations,
}

pub struct Rotations {
    pub ne: Vec<(i8, i8)>,
    pub ns: Vec<(i8, i8)>,
    pub nw: Vec<(i8, i8)>,

    pub es: Vec<(i8, i8)>,
    pub ew: Vec<(i8, i8)>,
    pub en: Vec<(i8, i8)>,

    pub sw: Vec<(i8, i8)>,
    pub sn: Vec<(i8, i8)>,
    pub se: Vec<(i8, i8)>,

    pub wn: Vec<(i8, i8)>,
    pub we: Vec<(i8, i8)>,
    pub ws: Vec<(i8, i8)>,
}

pub fn parse(s: &str) -> Option<Vec<Physics>> {
    #[derive(Deserialize)]
    struct PieceInfo {
        name: String,
        color: u8,
        minoes: Vec<(u8, u8)>,
        rotations: RotationsInfo,
    }

    #[derive(Deserialize, Debug)]
    struct RotationsInfo {
        ne: Option<Vec<(i8, i8)>>,
        ns: Option<Vec<(i8, i8)>>,
        nw: Option<Vec<(i8, i8)>>,

        es: Option<Vec<(i8, i8)>>,
        ew: Option<Vec<(i8, i8)>>,
        en: Option<Vec<(i8, i8)>>,

        sw: Option<Vec<(i8, i8)>>,
        sn: Option<Vec<(i8, i8)>>,
        se: Option<Vec<(i8, i8)>>,

        wn: Option<Vec<(i8, i8)>>,
        we: Option<Vec<(i8, i8)>>,
        ws: Option<Vec<(i8, i8)>>,
    }

    let infos: Vec<PieceInfo> = miniserde::json::from_str(s).ok()?;
    let mut physics: Vec<Physics> = Vec::new();

    for info in infos {
        if info.color == 0 {
            return None;
        }

        let min_x: u8 = info.minoes.iter().map(|(x, _)| *x).min()?;
        let min_y: u8 = info.minoes.iter().map(|(_, y)| *y).min()?;

        if min_x != 0 || min_y != 0 {
            return None;
        }

        let w: u8 = info.minoes.iter().map(|(x, _)| *x).max()?;
        let h: u8 = info.minoes.iter().map(|(_, y)| *y).max()?;

        let mut minoes_n = info.minoes;
        let mut minoes_e: Vec<_> = minoes_n.iter().map(|(x, y)| (*y, w - x)).collect();
        let mut minoes_s: Vec<_> = minoes_n.iter().map(|(x, y)| (w - x, h - y)).collect();
        let mut minoes_w: Vec<_> = minoes_n.iter().map(|(x, y)| (h - y, *x)).collect();
        minoes_n.sort();
        minoes_e.sort();
        minoes_s.sort();
        minoes_w.sort();

        let mut rotations = info.rotations;
        opposing(&mut rotations.ne, &mut rotations.en);
        opposing(&mut rotations.ns, &mut rotations.sn);
        opposing(&mut rotations.nw, &mut rotations.wn);
        opposing(&mut rotations.es, &mut rotations.se);
        opposing(&mut rotations.ew, &mut rotations.we);
        opposing(&mut rotations.sw, &mut rotations.ws);

        fn opposing(a: &mut Option<Vec<(i8, i8)>>, b: &mut Option<Vec<(i8, i8)>>) {
            let invert = |v: &[(i8, i8)]| v.iter().map(|(x, y)| (-x, -y)).collect();

            if a.is_some() && b.is_none() {
                *b = Some(invert(a.as_ref().unwrap()));
            } else if a.is_none() && b.is_some() {
                *a = Some(invert(b.as_ref().unwrap()));
            }
        }

        let rotations = Rotations {
            ne: rotations.ne.unwrap_or_default(),
            ns: rotations.ns.unwrap_or_default(),
            nw: rotations.nw.unwrap_or_default(),
            es: rotations.es.unwrap_or_default(),
            ew: rotations.ew.unwrap_or_default(),
            en: rotations.en.unwrap_or_default(),
            sw: rotations.sw.unwrap_or_default(),
            sn: rotations.sn.unwrap_or_default(),
            se: rotations.se.unwrap_or_default(),
            wn: rotations.wn.unwrap_or_default(),
            we: rotations.we.unwrap_or_default(),
            ws: rotations.ws.unwrap_or_default(),
        };

        physics.push(Physics {
            name: info.name,
            color: info.color,
            minoes: [minoes_n, minoes_e, minoes_s, minoes_w],
            width: w + 1,
            height: h + 1,
            rotations,
        });
    }

    let names: HashSet<&str> = physics.iter().map(|p| p.name.as_ref()).collect();
    if names.len() != physics.len() {
        return None;
    }

    Some(physics)
}
