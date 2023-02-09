use mysql::prelude::*;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;

pub type TableName = String;
pub type ColName = String;

pub struct Spec {
    // how to create fake users (which tables to insert which values)
    pub user_spec: Vec<TableSpec>,
    // src => dest links
    pub forward_links: HashMap<String, Link>,
    // dest => src links
    pub backward_links: HashMap<String, Link>,
}

impl Spec {
    pub fn new() -> Spec {
        Spec {
            user_spec: vec![],
            forward_links: HashMap::new(),
            backward_links: HashMap::new(),
        }
    }

    pub fn create_user<Q: Queryable>(&self, db: &mut Q) -> mysql::Result<()> {
        for ts in &self.user_spec {
            ts.insert_row(db)?;
        }
        Ok(())
    }

    // TODO DFS through graph to get connection from table to target
    pub fn query_linked(&self, target: TableName, src: TableName) -> String {
        let mut joins = String::new();
        match self.forward_links.get(&src) {
            Some(link) => joins.push_str(&format!(
                " {} ON {}.{} = {}.{}",
                link.src, link.src, link.src_fk, link.dest, link.dest_fk
            )),
            None => unimplemented!("Searching for path that doesn't exist?"),
        }
        let q = format!("SELECT * FROM {} JOIN {}", target, joins);
        q
    }
}

// INSERT INTO _ (col1, col2, ..) VALUES (v1,_,_)
pub struct TableSpec {
    table_name: TableName,
    columns: Vec<ColName>,
    values: Vec<ValueSpec>,
}

impl TableSpec {
    pub fn insert_row<Q: Queryable>(&self, db: &mut Q) -> mysql::Result<()> {
        let values: Vec<String> = self
            .values
            .iter()
            .map(|v| valuespec2value(v).as_sql(false))
            .collect();
        let q = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table_name,
            self.columns.join(","),
            values.join(",")
        );
        db.query_drop(q)
    }
}

// src.src_fk => dest.dest_fk
pub struct Link {
    src: TableName,
    dest: TableName,
    src_fk: ColName,
    dest_fk: ColName,
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
