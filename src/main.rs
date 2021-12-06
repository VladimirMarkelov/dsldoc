use std::env;
use std::process::exit;
use std::fs;
use std::path::Path;
use std::io::{self, BufRead, Cursor, Write};
use std::collections::HashMap;

use encoding_rs;

#[derive(Debug,PartialEq,Copy,Clone)]
enum DState {
    Begin,
    Name,
    Index,
    Lang,
    EmptyLine,
    Key,
    Comment,
    Text,
    M1,
    M2,
    P,
    I,
    C,
    B,
    Ex,
    RomanNumber,
    LangID,

    MClose,
    IClose,
    ComClose,
    PClose,
    CClose,
    BClose,
    ExClose,
    LangIDClose,

    Invalid,
}

fn load_utf16_file(filename: &str) -> String {
    let path = Path::new(filename);
    let bytes = match fs::read(path) {
        Err(e) => {
            println!("{}", e);
            exit(1);
        }
        Ok(v) => v,
    };
    let (res, _enc, _used) = encoding_rs::UTF_16LE.decode(&bytes);
    res.to_string()
}

fn roman_to_u8(s: &str) -> u8 {
    match s.trim() {
        "I" => 1,
        "II" => 2,
        "III" => 3,
        "IV" => 4,
        "V" => 5,
        "VI" => 6,
        "VII" => 7,
        "VIII" => 8,
        "IX" => 9,
        "X" => 10,
        _ => 0,
    }
}

fn tag_type(s: &str) -> DState {
    if s.starts_with("[lang id=") {
        return DState::LangID;
    }
    match s {
        "[com]" => DState::Comment,
        "[/com]" => DState::ComClose,
        "[m1]" => DState::M1,
        "[m2]" => DState::M2,
        "[/m]" => DState::MClose,
        "[p]" => DState::P,
        "[/p]" => DState::PClose,
        "[i]" => DState::I,
        "[/i]" => DState::IClose,
        "[ex]" => DState::Ex,
        "[/ex]" => DState::ExClose,
        "[c]" => DState::C,
        "[/c]" => DState::CClose,
        "[b]" => DState::B,
        "[/b]" => DState::BClose,
        "[/lang]" => DState::LangIDClose,
        _ => DState::Invalid,
    }
}

fn line_type(s: &str) -> DState {
    if s.is_empty() {
        return DState::EmptyLine;
    }
    if s.starts_with("#NAME ") {
        return DState::Name;
    }
    if s.starts_with("#INDEX_LANGUAGE ") {
        return DState::Index;
    }
    if s.starts_with("#CONTENTS_LANGUAGE ") {
        return DState::Lang;
    }
    if !s.starts_with('\t') {
        return DState::Key;
    }
    let tr = s.trim();
    if tr == "I" || tr == "II" || tr == "III" || tr == "IV" || tr == "V" || tr == "VI" || tr == "VII" || tr == "VIII" || tr == "IX" || tr == "X" {
        return DState::RomanNumber;
    }
    if !s.starts_with("\t[") {
        return DState::Text;
    }
    if s.starts_with("\t[m1]") {
        return DState::M1;
    }
    if s.starts_with("\t[m2]") {
        return DState::M2;
    }
    if s.contains("[com]") {
        return DState::Comment;
    }
    DState::Invalid
}

fn can_follow(prev: DState, curr: DState) -> bool {
    match prev {
        DState::Begin => curr == DState::Name,
        DState::Name => curr == DState::Index,
        DState::Index => curr == DState::Lang,
        DState::Lang => curr == DState::EmptyLine,
        DState::EmptyLine => curr == DState::Key || curr == DState::EmptyLine,
        DState::Key => curr == DState::Comment || curr == DState::Text || curr == DState::M1 || curr == DState::RomanNumber,
        DState::Comment => curr == DState::Text || curr == DState::M1 || curr == DState::Comment || curr == DState::RomanNumber || curr == DState::Key,
        DState::Text => curr == DState::Comment || curr == DState::Key || curr == DState::M1 || curr == DState::M2 || curr == DState::Text || curr == DState::RomanNumber,
        DState::M1 => curr == DState::Comment || curr == DState::Key || curr == DState::M1 || curr == DState::M2 || curr == DState::RomanNumber || curr == DState::Text,
        DState::M2 => curr == DState::Comment || curr == DState::Key || curr == DState::M1 || curr == DState::M2 || curr == DState::RomanNumber || curr == DState::Text,
        DState::RomanNumber => curr == DState::Comment || curr == DState::M1 || curr == DState::Text,
        _ => false,
    }
}

fn parse_line(s: &str) -> String {
    let s = s.trim();
    let mut in_sq = false;
    let mut tag = String::new();
    let mut stack: Vec<DState> = Vec::new();
    let mut last_c = ' ';
    for c in s.chars() {
        match c {
            '[' => {
                    if last_c == '\\' {
                        last_c = c;
                        continue;
                    }
                    if in_sq {
                        return format!("opening bracket inside tag: '{}['", tag);
                    }
                    tag = String::from("[");
                    in_sq = true;
            }
            ']' => {
                if last_c == '\\' {
                    if in_sq {
                        tag.push(c);
                    }
                    last_c = c;
                    continue;
                }
                if !in_sq {
                    return String::from("orphan closing bracket");
                }
                tag.push(c);
                in_sq = false;
                let tp = tag_type(&tag);
                if tp == DState::Invalid {
                    return format!("unknown tag '{}'", tag);
                }
                if !tag.starts_with("[/") {
                    stack.push(tp);
                } else {
                    if stack.is_empty() {
                        return format!("superfluos closing tag '{}'", tag);
                    }
                    let last = stack.pop().unwrap();
                    let matched = match tp {
                        DState::MClose if last == DState::M1 || last == DState::M2 => true,
                        DState::IClose if last == DState::I => true,
                        DState::ComClose if last == DState::Comment => true,
                        DState::PClose if last == DState::P => true,
                        DState::CClose if last == DState::C => true,
                        DState::BClose if last == DState::B => true,
                        DState::ExClose if last == DState::Ex => true,
                        DState::LangIDClose if last == DState::LangID => true,
                        _ => false,
                    };
                    if !matched {
                        return format!("opening tag '{:?}' closing '{:?}'", last, tp);
                    }
                    tag = String::new();
                }
            },
            _ => if in_sq {
                tag.push(c);
            },
        }
        last_c = c;
    }
    if !tag.is_empty() {
        return format!("unfinished tag '{}'", tag);
    }
    if !stack.is_empty() {
        return format!("unclosed tags: {:?}", stack);
    }
    String::new()
}

fn check_grammar(filename: &str) {
    let cont = load_utf16_file(filename);
    let cursor = Cursor::new(cont.as_bytes());
    let mut prev = DState::Begin;
    let mut words: HashMap<String, usize> = HashMap::new();

    for (idx, l) in cursor.lines().flatten().enumerate() {
        let tp = line_type(&l);
        if tp == DState::Invalid {
            println!("{:4}.{}", idx, l);
        }
        if tp == DState::Key {
            let mut exist = false;
            if let Some(v) = words.get(&l) {
                exist = true;
                println!("{} at {} already exists at {}", l, idx, *v);
            }
            if !exist {
                words.insert(l.clone(), idx);
            }
        }
        if !can_follow(prev, tp) {
            println!("{:4}.PREV {:?}, CURR: {:?}{}", idx, prev, tp, l);
        }
        let prs = parse_line(&l);
        if !prs.is_empty() {
            println!("{:4}.{} ==> {}", idx, prs, l);
        }
        prev = tp;
    }
}

fn fix_up_line(s: &str) -> String {
    let mut in_sq = false;
    let mut tag = String::new();
    let mut last_c = ' ';
    let mut res = String::new();
    for c in s.chars() {
        match c {
            '[' => {
                    if last_c == '\\' {
                        last_c = c;
                        res.push(c);
                        continue;
                    }
                    tag = String::from("[");
                    in_sq = true;
            }
            ']' => {
                if last_c == '\\' {
                    if in_sq {
                        tag.push(c);
                    } else {
                        res.push(c);
                    }
                    last_c = c;
                    continue;
                }
                if !in_sq {
                    res.push('\\');
                    res.push(c);
                    continue;
                }
                tag.push(c);
                in_sq = false;
                let tp = tag_type(&tag);
                if tp == DState::Invalid {
                    let t = tag.trim_end_matches(|c| c == ']');
                    let tg = format!("\\{}\\]", t);
                    res.push_str(&tg);
                    println!("replacing '{}' with '{}'", tag, tg);
                } else {
                    res.push_str(&tag);
                }
                tag = String::new();
            },
            _ => if in_sq {
                tag.push(c);
            } else {
                res.push(c);
            },
        }
        last_c = c;
    }
    if !tag.is_empty() {
        res.push_str(&tag);
    }
    res
}

// TODO:
fn fix_invalid_tags(infile: &str, outfile: &str) {
    let cont = load_utf16_file(infile);
    let cursor = Cursor::new(cont.as_bytes());
    let mut rvec: Vec<String> = Vec::new();
    for l in cursor.lines().flatten() {
        if !l.contains('[') {
            rvec.push(l.to_string());
            continue;
        }
        let prs = fix_up_line(&l);
        rvec.push(prs.to_string());
    }

    let mut f = fs::File::create(Path::new(&outfile)).unwrap();
    for s in rvec.iter() {
        writeln!(f, "{}", s).unwrap();
    }
}

// TODO:
fn sort_file(infile: &str, outfile: &str) {
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        println!("No arguments");
        println!("    check FILENAME\nCheck for valid tag order\n");
        println!("    fix-tags FILENAME OUT_FILENAME\nEscape square brackets for unknown tags (use only if check is OK)\n");
        return;
    } else if args.len() == 2 {
        println!("two arguments expected: [COMMAND] [FILE]");
        return;
    }
    let cmd = args[1].as_str();
    let filename = args[2].as_str();
    println!("{} --> {}", cmd, filename);

    match cmd {
        "check" => check_grammar(filename),
        "fix-tags" => if args.len() < 4 {
            println!("output filename is undefined");
        } else {
            fix_invalid_tags(filename, &args[3]);
        },
        "sort" => if args.len() < 4 {
            println!("output filename is undefined");
        } else {
            sort_file(filename, &args[3]);
        },
        _ => println!("invalid command: {}", cmd),
    }
}
