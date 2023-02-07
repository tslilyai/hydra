use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;

pub struct OwnershipSpec {
    // how to create fake users (which tables to insert which values)
    user_spec: Vec<TableSpec>,
    // src table => link type to users
    data_links: HashMap<String, Link>,
}

pub struct TableSpec {
    // INSERT INTO _ (col1, col2, ..) VALUES (v1,_,_)
    table_name: String,
    columns: Vec<String>,
    values: Vec<ValueSpec>,
}

pub struct Link {
    src: String,
    dest: String,
    fk: String,
}

pub enum LinkType {
    Direct(Link),
    Indirect(Vec<Link>),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ValueSpec {
    ConstNum(u64),
    ConstStr(String),
    RandNum { lb: usize, ub: usize },
    RandStr { len: usize },
    RandEmail,
    RandPhone,
    ConstDate { year: u16, month: u8, day: u8 },
    Bool(bool),
    Null,
}

pub fn valuespec2value(vs: &ValueSpec) -> mysql::Value {
    use mysql::Value::*;
    use ValueSpec::*;
    match vs {
        ConstNum(n) => UInt(*n),
        ConstStr(s) => Bytes(s.clone().into_bytes()),
        RandNum { lb, ub } => {
            let mut rng = rand::thread_rng();
            UInt(rng.gen_range(*lb..*ub).try_into().unwrap())
        }
        RandStr { len } => {
            let rng = rand::thread_rng();
            let rand_string: String = rng
                .sample_iter(&Alphanumeric)
                .take(*len)
                .map(char::from)
                .collect();
            Bytes(rand_string.into_bytes())
        }
        RandEmail => {
            let rng = rand::thread_rng();
            let rand_string: String = rng
                .sample_iter(&Alphanumeric)
                .take(20)
                .map(char::from)
                .collect();
            Bytes(format!("{}@anon.com", rand_string).into_bytes())
        }
        RandPhone => {
            let mut rng = rand::thread_rng();
            const CHARSET: &[u8] = b"0123456789";
            const LEN: usize = 9;
            let rand_phone: String = (0..LEN)
                .map(|_| {
                    let idx = rng.gen_range(0..CHARSET.len());
                    CHARSET[idx] as char
                })
                .collect();
            Bytes(rand_phone.into_bytes())
        }
        Bool(b) => match b {
            true => Int(1),
            false => Int(0),
        },
        ConstDate { year, month, day } => Date(*year, *month, *day, 0, 0, 0, 0),
        Null => NULL,
    }
}
