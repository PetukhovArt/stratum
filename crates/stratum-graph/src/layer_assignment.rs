use camino::Utf8Path;

use stratum_core::ids::LayerId;
use stratum_core::types::Layer;

/// Assign a module path to a `Layer` by longest-matching-prefix.
///
/// Layers with non-UTF-8 paths are skipped. Ties are broken by deeper path
/// (more path components wins).
#[must_use]
pub fn assign_layer(layers: &[Layer], module_path: &Utf8Path) -> Option<LayerId> {
    let mut best: Option<(usize, LayerId)> = None;
    for layer in layers {
        let Some(lp_str) = layer.path.to_str() else {
            continue;
        };
        let lp = Utf8Path::new(lp_str);
        if module_path.starts_with(lp) {
            let depth = lp.components().count();
            match best {
                Some((best_depth, _)) if best_depth >= depth => {}
                _ => best = Some((depth, layer.id)),
            }
        }
    }
    best.map(|(_, id)| id)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn layer(id: u32, name: &str, path: &str) -> Layer {
        Layer {
            id: LayerId::new(id),
            name: name.into(),
            path: PathBuf::from(path),
            depends_on: vec![],
        }
    }

    #[test]
    fn longest_prefix_wins() {
        let layers = vec![
            layer(0, "app", "src/app"),
            layer(1, "features", "src/features"),
            layer(2, "shared", "src/shared"),
            layer(3, "shared-ui", "src/shared/ui"),
        ];
        let got = assign_layer(&layers, Utf8Path::new("src/shared/ui/Button.tsx"));
        assert_eq!(got, Some(LayerId::new(3)));
    }

    #[test]
    fn no_match_returns_none() {
        let layers = vec![layer(0, "app", "src/app")];
        let got = assign_layer(&layers, Utf8Path::new("src/features/cart/index.ts"));
        assert_eq!(got, None);
    }

    #[test]
    fn exact_match() {
        let layers = vec![layer(0, "shared", "src/shared")];
        let got = assign_layer(&layers, Utf8Path::new("src/shared/api/http.ts"));
        assert_eq!(got, Some(LayerId::new(0)));
    }
}
