use std::env;

pub struct Flags {
    pub json: bool,
    pub debug: bool,
    pub serial: Option<String>,
}

pub fn parse_flags(args: &[String]) -> Flags {
    let mut flags = Flags {
        json: false,
        debug: false,
        serial: env::var("ANDROID_SERIAL").ok(),
    };

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => flags.json = true,
            "--debug" => flags.debug = true,
            "--serial" | "-s" | "--session" => {
                if let Some(s) = args.get(i + 1) {
                    flags.serial = Some(s.clone());
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    flags
}

pub fn clean_args(args: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    let mut skip_next = false;

    // Global flags that should be stripped from command args
    const GLOBAL_FLAGS: &[&str] = &["--json", "--debug"];
    // Global flags that take a value (need to skip the next arg too)
    const GLOBAL_FLAGS_WITH_VALUE: &[&str] = &["--serial", "-s", "--session"];

    for arg in args.iter() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if GLOBAL_FLAGS_WITH_VALUE.contains(&arg.as_str()) {
            skip_next = true;
            continue;
        }
        if GLOBAL_FLAGS.contains(&arg.as_str()) {
            continue;
        }
        result.push(arg.clone());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn test_parse_flags() {
        let flags = parse_flags(&args("--json --debug -s device123"));
        assert!(flags.json);
        assert!(flags.debug);
        assert_eq!(flags.serial, Some("device123".to_string()));
    }

    #[test]
    fn test_clean_args() {
        let cleaned = clean_args(&args("--json -s device123 install app.apk"));
        assert_eq!(cleaned, vec!["install", "app.apk"]);
    }
}
