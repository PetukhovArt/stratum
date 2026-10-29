use super::Declared;
use super::scan::{Scan, Token};

/// Each `rpc M(…) returns (…)` directly inside a `service S { … }` is `S.M`.
pub(super) fn contracts(text: &str) -> Vec<Declared> {
    let scan = Scan::new(text);
    let tokens = scan.tokens(&['.']);
    let mut found = Vec::new();
    let mut depth = 0usize;
    let mut service: Option<(&str, usize)> = None;
    let mut i = 0;
    while let Some(token) = tokens.get(i) {
        match token.text {
            "{" => depth += 1,
            "}" => {
                depth = depth.saturating_sub(1);
                if service.is_some_and(|(_, body)| depth < body) {
                    service = None;
                }
            }
            "service" => {
                if let [name, open, ..] = &tokens[i + 1..] {
                    if open.is("{") {
                        service = Some((name.text, depth + 1));
                    }
                }
            }
            "rpc" => {
                if let Some((name, _)) = service.filter(|&(_, body)| body == depth) {
                    if let Some((method, close)) = rpc(&tokens[i..]) {
                        found.push(Declared {
                            id: format!("{name}.{method}"),
                            signature: scan.squeezed(token.start, close.end()),
                            line: scan.line(token.start),
                        });
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    found
}

/// The method name of `rpc M ( … ) returns ( … )` and its last `)`.
fn rpc<'s>(tokens: &[Token<'s>]) -> Option<(&'s str, Token<'s>)> {
    let name = tokens.get(1)?;
    let returns = tokens.iter().take(32).position(|t| t.is("returns"))?;
    let close = tokens[returns..].iter().find(|t| t.is(")"))?;
    Some((name.text, *close))
}

#[cfg(test)]
mod tests {
    use super::contracts;

    #[test]
    fn rpcs_of_each_service_with_stream_options_comments_and_split_lines() {
        let text = r#"syntax = "proto3";
// service Commented { rpc Nope(A) returns (B); }
service UserService {
  rpc GetUser(GetUserRequest) returns (User);
  /* rpc Hidden(A) returns (B); */
  rpc WatchUsers(stream WatchRequest)
      returns (stream User) {
    option (google.api.http) = { get: "/v1/{name=users/*}" };
  }
}
message User { string id = 1; }
service OrderService { rpc PlaceOrder(pkg.Order) returns (pkg.Receipt); }
"#;
        let found: Vec<(String, String, u32)> = contracts(text)
            .into_iter()
            .map(|c| (c.id, c.signature, c.line))
            .collect();
        assert_eq!(
            found,
            [
                (
                    "UserService.GetUser".into(),
                    "rpc GetUser(GetUserRequest) returns (User)".into(),
                    4
                ),
                (
                    "UserService.WatchUsers".into(),
                    "rpc WatchUsers(stream WatchRequest) returns (stream User)".into(),
                    6
                ),
                (
                    "OrderService.PlaceOrder".into(),
                    "rpc PlaceOrder(pkg.Order) returns (pkg.Receipt)".into(),
                    12
                ),
            ]
        );
    }
}
