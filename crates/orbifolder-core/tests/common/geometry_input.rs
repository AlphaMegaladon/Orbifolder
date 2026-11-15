//! Legacy text/Q(sqrt(3)) adapter, outside the in-memory core and oracle output.
use crate::common::root;
use orbifolder_core::{
    geometry::{DiscreteTorsion, ElementSpec, EmbeddingSpec, GeneratorSpec, GeometrySpec, Limits},
    math_core::{GaugeLattice, Matrix, Rational, RightWeight, TorusVector, Vector},
};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn section<'a>(lines: &[&'a str], begin: &str, end: &str) -> Vec<&'a str> {
    let i = lines.iter().position(|&x| x == begin).unwrap() + 1;
    lines[i..]
        .iter()
        .take_while(|&&x| x != end)
        .copied()
        .filter(|x| !x.is_empty())
        .collect()
}
fn scalar(value: &str) -> (Rational, Rational) {
    if value == "1.7320508076" {
        return (Rational::ZERO, Rational::ONE);
    }
    if value == "-1.7320508076" {
        return (Rational::ZERO, -Rational::ONE);
    }
    (value.trim_end_matches('.').parse().unwrap(), Rational::ZERO)
}
fn element(line: &str, susy: bool) -> ElementSpec {
    let values = line
        .split_whitespace()
        .map(|x| x.parse::<Rational>().unwrap())
        .collect::<Vec<_>>();
    let factors = if susy {
        [
            values[0].to_integer().unwrap().try_into().unwrap(),
            values[1].to_integer().unwrap().try_into().unwrap(),
            0,
        ]
    } else {
        std::array::from_fn(|i| values[i].to_integer().unwrap().try_into().unwrap())
    };
    ElementSpec {
        factors,
        translation: TorusVector::from_slice(&values[if susy { 2 } else { 3 }..]).unwrap(),
    }
}
pub fn from_fixture(data: &Value) -> GeometrySpec {
    from_text(
        data["variant"].as_str().unwrap(),
        data["input"]["model_text"].as_str().unwrap(),
        Some(data["input"]["geometry_sha256"].as_str().unwrap()),
    )
}
pub fn geometry_text(variant: &str, model: &str, geometry_hash: Option<&str>) -> String {
    let file = model
        .lines()
        .find_map(|x| x.strip_prefix("SpaceGroup:"))
        .unwrap();
    let inputs: Value =
        serde_json::from_slice(&std::fs::read(root().join("geometry_inputs.json")).unwrap())
            .unwrap();
    let entry = &inputs[format!("{variant}/{file}")];
    let text = entry["text"].as_str().expect("bundled geometry input");
    let bytes = text.as_bytes();
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        entry["sha256"].as_str().unwrap()
    );
    if let Some(hash) = geometry_hash {
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), hash);
    }
    text.to_owned()
}
pub fn from_text(variant: &str, model: &str, geometry_hash: Option<&str>) -> GeometrySpec {
    let text = geometry_text(variant, model, geometry_hash);
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|x| !x.starts_with("//"))
        .collect::<Vec<_>>();
    let susy = variant == "susy";
    let pg = lines.iter().position(|&x| x == "point group").unwrap() + 1;
    let orders = if susy {
        [
            lines[pg].parse().unwrap(),
            lines[pg + 1].parse().unwrap(),
            1,
        ]
    } else {
        std::array::from_fn(|i| lines[pg + i].parse().unwrap())
    };
    let twist_lines = section(&lines, "begin twist", "end twist");
    let translations = section(
        &lines,
        "begin shift space group generators",
        "end shift space group generators",
    );
    assert_eq!(translations.len(), 6);
    for (i, line) in translations.iter().enumerate() {
        let g = element(line, susy);
        assert_eq!(g.factors, [0; 3]);
        assert_eq!(
            g.translation,
            TorusVector::from_integers(std::array::from_fn(|j| i128::from(i == j)))
        );
    }
    let mut twists = std::array::from_fn::<_, 3, _>(|_| RightWeight::zero());
    for (i, line) in twist_lines.iter().enumerate() {
        twists[i] = RightWeight::from_slice(
            &line
                .split_whitespace()
                .map(|x| x.parse().unwrap())
                .collect::<Vec<_>>(),
        )
        .unwrap();
    }
    let basis = section(&lines, "begin lattice", "end lattice")
        .into_iter()
        .filter(|&x| x != "next lattice vector")
        .map(scalar)
        .collect::<Vec<_>>();
    assert_eq!(basis.len(), 36);
    let gram = Matrix::new(
        6,
        6,
        (0..6)
            .flat_map(|i| (0..6).map(move |j| (i, j)))
            .map(|(i, j)| {
                let mut real = Rational::ZERO;
                let mut irrational = Rational::ZERO;
                for k in 0..6 {
                    let (a, b) = &basis[i * 6 + k];
                    let (c, d) = &basis[j * 6 + k];
                    real = real + a * c + Rational::from(3) * b * d;
                    irrational = irrational + a * d + b * c;
                }
                assert!(irrational.is_zero());
                real
            })
            .collect(),
    )
    .unwrap();
    let gen_lines = section(
        &lines,
        "begin twist space group generators",
        "end twist space group generators",
    );
    let mut translations = std::array::from_fn::<_, 3, _>(|_| TorusVector::zero());
    for line in gen_lines {
        let g = element(line, susy);
        let i = g.factors.iter().position(|&x| x == 1).unwrap();
        translations[i] = g.translation;
    }
    let generators = std::array::from_fn(|i| {
        let mut entries = vec![Rational::ZERO; 36];
        for plane in 0..3 {
            let half = Rational::new(1, 2).unwrap();
            let offset = 2 * plane;
            let a2 = basis[offset * 6 + offset] == (Rational::from(2), Rational::ZERO)
                && basis[(offset + 1) * 6 + offset + 1] == (Rational::ZERO, Rational::ONE);
            let phase = twists[i].coordinates()[plane + 1].modulo_one();
            let (angle, base) = if a2 {
                (phase * Rational::from(6), vec![1, -1, 1, 0])
            } else {
                assert_eq!(basis[offset * 6 + offset], (Rational::ZERO, Rational::ONE));
                assert!(phase.is_zero() || phase == half);
                (phase * Rational::from(4), vec![0, -1, 1, 0])
            };
            let power: u32 = angle.to_integer().unwrap().try_into().unwrap();
            let base = Matrix::new(2, 2, base.into_iter().map(Rational::from).collect()).unwrap();
            let mut action = Matrix::identity(2).unwrap();
            for _ in 0..power {
                action = action.multiply(&base).unwrap();
            }
            for row in 0..2 {
                for col in 0..2 {
                    entries[(offset + row) * 6 + offset + col] =
                        action.get(row, col).unwrap().clone();
                }
            }
        }
        GeneratorSpec {
            order: orders[i],
            rotation: Matrix::new(6, 6, entries).unwrap(),
            translation: translations[i].clone(),
            twist: twists[i].clone(),
        }
    });
    let mut representatives: Vec<Vec<ElementSpec>> = Vec::new();
    for line in section(
        &lines,
        "begin constructing elements",
        "end constructing elements",
    ) {
        if line == "new sector" {
            representatives.push(Vec::new());
        } else {
            representatives
                .last_mut()
                .unwrap()
                .push(element(line, susy));
        }
    }
    let mut presentations: Vec<Vec<ElementSpec>> = Vec::new();
    for line in section(
        &lines,
        "begin centralizer elements",
        "end centralizer elements",
    ) {
        if line == "begin data" {
            presentations.push(Vec::new());
        } else if line != "end data" {
            presentations.last_mut().unwrap().push(element(line, susy));
        }
    }
    GeometrySpec {
        gram,
        generators,
        representatives: Some(representatives),
        centralizer_presentations: Some(presentations),
        limits: Limits::default(),
    }
}

pub fn embedding(variant: &str, model: &str) -> EmbeddingSpec {
    let rows = model
        .split("Shifts and Wilsonlines:\n")
        .nth(1)
        .unwrap()
        .lines()
        .take_while(|line| !line.starts_with("end model"))
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            Vector::from_slice(
                &line
                    .split_whitespace()
                    .map(|x| x.parse::<Rational>().unwrap())
                    .collect::<Vec<_>>(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let count = if variant == "susy" { 2 } else { 3 };
    assert_eq!(rows.len(), count + 6);
    let shifts = std::array::from_fn(|i| {
        if i < count {
            rows[i].clone()
        } else {
            Vector::zero()
        }
    });
    let wilson_lines = std::array::from_fn(|i| rows[count + i].clone());
    let lattice = if model.lines().any(|x| x == "Lattice:Spin32") {
        GaugeLattice::D16Plus
    } else {
        GaugeLattice::E8xE8
    };
    EmbeddingSpec {
        label: model
            .lines()
            .find_map(|x| x.strip_prefix("Label:"))
            .unwrap()
            .to_string(),
        lattice,
        shifts,
        wilson_lines,
        torsion: DiscreteTorsion::default(),
    }
}
