use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::Result;

const LAYERS: [&str; 6] = ["shared", "entities", "features", "widgets", "pages", "app"];
const SHARES: [usize; 6] = [20, 22, 30, 14, 10, 4];
const SLICE_PREFIX: [&str; 6] = ["", "entity", "feature", "widget", "page", ""];
const SEGMENTS: [&str; 5] = ["model", "api", "lib", "ui", "_shared"];
const SLICE_SIZE: usize = 12;
pub const MODULES: usize = 4000;
const MAIN: &str = "src/app/main.ts";

struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap_or_default() % n.max(1)
    }
}

struct Module {
    path: String,
    unit: usize,
    vue: bool,
    imports: Vec<(usize, String)>,
}

struct Unit {
    layer: usize,
    dir: String,
    members: Vec<usize>,
    entry: bool,
    public: Vec<usize>,
}

struct Plan {
    modules: Vec<Module>,
    units: Vec<Unit>,
    by_layer: Vec<Vec<usize>>,
}

impl Plan {
    fn new(total: usize) -> Self {
        let mut plan = Self {
            modules: Vec::new(),
            units: Vec::new(),
            by_layer: vec![Vec::new(); LAYERS.len()],
        };
        for (layer, share) in SHARES.iter().enumerate() {
            let count = total * share / 100;
            match layer {
                0 => plan.flat(layer, &["ui", "lib", "api", "config"], count),
                5 => {
                    let main = plan.unit(layer, "src/app".into(), false);
                    plan.module(main, MAIN.into(), false);
                    plan.flat(layer, &["providers", "router", "store"], count);
                }
                _ => plan.sliced(layer, count),
            }
        }
        plan
    }

    fn unit(&mut self, layer: usize, dir: String, entry: bool) -> usize {
        self.units.push(Unit {
            layer,
            dir,
            members: Vec::new(),
            entry,
            public: Vec::new(),
        });
        self.by_layer[layer].push(self.units.len() - 1);
        self.units.len() - 1
    }

    fn module(&mut self, unit: usize, path: String, vue: bool) -> usize {
        let id = self.modules.len();
        self.modules.push(Module {
            path,
            unit,
            vue,
            imports: Vec::new(),
        });
        self.units[unit].members.push(id);
        id
    }

    fn flat(&mut self, layer: usize, dirs: &[&str], count: usize) {
        for (d, name) in dirs.iter().enumerate() {
            let unit = self.unit(layer, format!("src/{}/{name}", LAYERS[layer]), false);
            let size = count / dirs.len() + usize::from(d < count % dirs.len());
            for j in 0..size {
                let path = format!("src/{}/{name}/item-{j}.ts", LAYERS[layer]);
                self.module(unit, path, false);
            }
        }
    }

    fn sliced(&mut self, layer: usize, count: usize) {
        let slices = (count / SLICE_SIZE).max(1);
        for s in 0..slices {
            let dir = format!("src/{}/{}-{s:03}", LAYERS[layer], SLICE_PREFIX[layer]);
            let unit = self.unit(layer, dir.clone(), true);
            let size = count / slices + usize::from(s < count % slices);
            self.module(unit, format!("{dir}/index.ts"), false);
            for j in 1..size {
                let segment = SEGMENTS[(j - 1) % SEGMENTS.len()];
                let vue = segment == "ui" && j == 4 && s % 10 == 0;
                let ext = if vue { "vue" } else { "ts" };
                self.module(unit, format!("{dir}/{segment}/part-{j}.{ext}"), vue);
            }
            let members = &self.units[unit].members;
            let public = members[1..members.len().min(4)].to_vec();
            self.units[unit].public = public;
        }
    }

    fn pick_member(&self, unit: usize, rng: &mut Lcg, skip_entry: bool) -> Option<usize> {
        let u = &self.units[unit];
        let first = usize::from(u.entry && skip_entry);
        let candidates = &u.members[first..];
        if candidates.is_empty() {
            return None;
        }
        (0..4)
            .map(|_| candidates[rng.below(candidates.len())])
            .find(|&id| !self.modules[id].vue && self.modules[id].path != MAIN)
    }

    fn pick_target(&self, layer: usize, deep: bool, rng: &mut Lcg) -> Option<(usize, String)> {
        let unit = self.by_layer[layer][rng.below(self.by_layer[layer].len())];
        if !self.units[unit].entry {
            let id = self.pick_member(unit, rng, false)?;
            return Some((id, alias(&self.modules[id].path)));
        }
        if deep {
            let id = self.pick_member(unit, rng, true)?;
            return Some((id, alias(&self.modules[id].path)));
        }
        let public = &self.units[unit].public;
        let id = *public.get(rng.below(public.len().max(1)))?;
        Some((id, alias(&self.units[unit].dir)))
    }

    fn pick_leaf(&self, layer: usize, rng: &mut Lcg) -> Option<(usize, String)> {
        let unit = self.by_layer[layer][rng.below(self.by_layer[layer].len())];
        let id = *self.units[unit].members.last()?;
        (!self.modules[id].vue).then(|| (id, alias(&self.modules[id].path)))
    }

    fn link(&mut self, rng: &mut Lcg) {
        for id in 0..self.modules.len() {
            let imports = self.imports_of(id, rng);
            self.modules[id].imports = imports;
        }
        for unit in &self.units {
            if unit.entry && unit.members.len() > 4 && unit.dir.ends_with("00") {
                let (a, b) = (unit.members[2], unit.members[3]);
                for (from, to) in [(a, b), (b, a)] {
                    let spec = relative(&self.modules[from].path, &self.modules[to].path);
                    self.modules[from].imports.push((to, spec));
                }
            }
        }
    }

    fn imports_of(&self, id: usize, rng: &mut Lcg) -> Vec<(usize, String)> {
        let m = &self.modules[id];
        let unit = &self.units[m.unit];
        let layer = unit.layer;
        let pos = unit.members.iter().position(|&x| x == id).unwrap_or(0);
        if unit.entry && pos == 0 {
            return Vec::new();
        }
        if m.path == MAIN {
            let pages = self.by_layer[4]
                .iter()
                .filter_map(|&u| self.units[u].public.first().copied())
                .map(|t| (t, alias(&self.units[self.modules[t].unit].dir)));
            let registry = self.by_layer[5]
                .iter()
                .flat_map(|&u| &self.units[u].members)
                .filter(|&&t| t != id)
                .map(|&t| (t, relative(&m.path, &self.modules[t].path)));
            return pages.chain(registry).collect();
        }
        if unit.entry && pos == unit.members.len() - 1 {
            return Vec::new();
        }
        let mut found: Vec<(usize, String)> = Vec::new();
        for _ in 0..=rng.below(3) {
            let roll = rng.below(100);
            let after = &unit.members[pos + 1..];
            let target = if roll < 55 {
                (!after.is_empty())
                    .then(|| after[rng.below(after.len())])
                    .filter(|&t| !self.modules[t].vue)
                    .map(|t| (t, relative(&m.path, &self.modules[t].path)))
            } else if layer == 0 {
                None
            } else if roll < 84 {
                self.pick_target(rng.below(layer), false, rng)
            } else if roll < 88 {
                self.pick_target(rng.below(layer), true, rng)
            } else if roll < 89 && layer < 4 {
                self.pick_leaf(layer + 1 + rng.below(4 - layer), rng)
            } else if roll < 91 {
                self.pick_target(layer, false, rng)
                    .filter(|&(t, _)| self.modules[t].unit > m.unit)
            } else {
                self.pick_target(0, false, rng)
            };
            if let Some((t, spec)) = target.filter(|&(t, _)| t != id) {
                if found.iter().all(|&(f, _)| f != t) {
                    found.push((t, spec));
                }
            }
        }
        found
    }

    fn write(&self, root: &Path) -> Result<()> {
        for (id, m) in self.modules.iter().enumerate() {
            let unit = &self.units[m.unit];
            let mut src = String::new();
            if unit.entry && unit.members[0] == id {
                for &p in &unit.public {
                    let spec = relative(&m.path, &self.modules[p].path);
                    writeln!(src, "export * from '{spec}';")?;
                }
            } else {
                for (t, spec) in &m.imports {
                    writeln!(src, "import {{ m{t} }} from '{spec}';")?;
                }
                let sum: Vec<String> = m.imports.iter().map(|(t, _)| format!("m{t}")).collect();
                let sum = if sum.is_empty() {
                    "0".into()
                } else {
                    sum.join(" + ")
                };
                if m.vue {
                    src = format!(
                        "<script setup lang=\"ts\">\n{src}const total = {sum};\n</script>\n\n<template>\n  <div>{{{{ total }}}}</div>\n</template>\n"
                    );
                } else {
                    writeln!(src, "export const m{id} = {sum} + 1;")?;
                }
            }
            write_file(&root.join(&m.path), &src)?;
        }
        Ok(())
    }
}

fn alias(path: &str) -> String {
    let path = path.strip_prefix("src/").unwrap_or(path);
    let path = path.strip_suffix(".ts").unwrap_or(path);
    format!("@/{path}")
}

fn relative(from: &str, to: &str) -> String {
    let from_dir: Vec<&str> = from.split('/').collect();
    let to: Vec<&str> = to.split('/').collect();
    let from_dir = &from_dir[..from_dir.len() - 1];
    let common = from_dir.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let ups = from_dir.len() - common;
    let prefix = if ups == 0 {
        "./".to_owned()
    } else {
        "../".repeat(ups)
    };
    let rest = to[common..].join("/");
    format!("{prefix}{}", rest.strip_suffix(".ts").unwrap_or(&rest))
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(fs::write(path, content)?)
}

fn git(dir: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=budget",
            "-c",
            "user.email=budget@example.com",
        ])
        .args(["-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
        .args(args)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("git {args:?} failed").into())
    }
}

pub fn generate(dir: &Path, modules: usize) -> Result<()> {
    let mut plan = Plan::new(modules);
    plan.link(&mut Lcg(0x5712_a7e5));
    plan.write(dir)?;
    write_file(
        &dir.join("package.json"),
        "{ \"name\": \"budget-app\", \"private\": true }\n",
    )?;
    write_file(
        &dir.join("tsconfig.json"),
        "{\n  \"compilerOptions\": {\n    \"strict\": true,\n    \"module\": \"esnext\",\n    \"moduleResolution\": \"bundler\",\n    \"paths\": { \"@/*\": [\"./src/*\"] }\n  }\n}\n",
    )?;
    write_file(
        &dir.join("stratum.config.jsonc"),
        &format!(
            "{{\n  \"version\": 1,\n  \"project\": {{ \"extends\": \"stratum:fsd\", \"entry_points\": [\"{MAIN}\"] }},\n  \"rules\": {{\n    \"stratum/unused-exports\": {{ \"severity\": \"warning\" }},\n    \"stratum/unreachable-modules\": {{ \"severity\": \"warning\" }}\n  }}\n}}\n"
        ),
    )?;
    git(dir, &["init", "-q"])?;
    git(dir, &["add", "."])?;
    git(dir, &["commit", "-q", "-m", "budget project"])
}
