use crate::crypto::*;
use crate::spec::UID;
use crypto_box::{PublicKey, SecretKey};
use log::debug;
use num_bigint::BigInt;
use num_primes::Generator;
use pbkdf2::{
    password_hash::{PasswordHash, PasswordHasher, SaltString},
    Pbkdf2,
};
use rand::distributions::{Distribution, Uniform};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::convert::TryInto;
use std::hash::{Hash, Hasher};

pub type Share = [BigInt; 2];
pub type ShareValue = BigInt;
pub type ShareIndex = u64;

#[derive(Clone)]
pub struct UserCreds {
    pub pubkey: Option<PublicKey>,
    pub is_anon: bool,
    pub enc_locators_index: u64,
}

pub struct Authorizer {
    user_creds: HashMap<UID, UserCreds>,
    shares_map: HashMap<ShareIndex, ShareStore>,
    prime: BigInt,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareStore {
    pub share: Share,
    pub share_value: ShareValue,
    pub password_salt: String,
}

pub fn generate_keypair() -> (SecretKey, PublicKey) {
    let mut rng = rand::thread_rng();
    let secretkey = SecretKey::generate(&mut rng);
    let pubkey = PublicKey::from(&secretkey);
    debug!(
        "secretkey {} and pubkey {}",
        base64::encode(secretkey.as_bytes()),
        base64::encode(pubkey.as_bytes()),
    );
    let pk_hash = {
        let mut hasher = DefaultHasher::new();
        (base64::encode(secretkey.as_bytes())).hash(&mut hasher);
        hasher.finish()
    };
    (secretkey, pubkey)
}

impl Authorizer {
    pub fn new() -> Authorizer {
        let prime_arr: [u8; 64];
        loop {
            let prime_gen = Generator::new_prime(512).to_bytes_le();
            let try_pa: std::result::Result<[u8; 64], _> = prime_gen.try_into();
            match try_pa {
                Ok(p) => {
                    prime_arr = p;
                    break;
                }
                Err(_) => continue,
            }
        }

        Authorizer {
            user_creds: HashMap::new(),
            shares_map: HashMap::new(),
            prime: BigInt::from_bytes_le(num_bigint::Sign::Plus, &prime_arr),
        }
    }

    pub fn register_user_shares(&mut self, uid: &UID, password: &str) -> (Share, ShareIndex) {
        let (secretkey, _pubkey) = generate_keypair();

        let salt = SaltString::generate(&mut OsRng);
        let pass_info: String = Pbkdf2
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        let _parsed_hash = PasswordHash::new(&pass_info).unwrap();
        let hash_pass_bigint = BigInt::from_bytes_le(num_bigint::Sign::Plus, pass_info.as_bytes());
        let secretkey_int = BigInt::from_bytes_le(num_bigint::Sign::Plus, secretkey.as_bytes());

        let sss = ShamirSecretSharing {
            threshold: 1,
            share_count: 3,
            prime: self.prime.clone(),
        };
        // returned format: vec < [h(p), f(h(p))], [rand1, f(rand1)], [rand2, f(rand2)] >
        let all_shares = sss.share(&secretkey_int, &hash_pass_bigint);

        let mut uid_owned = uid.clone().to_owned();
        uid_owned.push_str(&password);
        let uid_pw_hash = {
            let mut hasher = DefaultHasher::new();
            uid_owned.hash(&mut hasher);
            hasher.finish()
        };
        debug!(
            "got uid, password {} {}: hash {}",
            uid, password, uid_pw_hash
        );

        // save share
        let perm_share = ShareStore {
            share: all_shares[1].clone(),
            share_value: all_shares[0][1].clone(),
            password_salt: salt.clone().as_str().to_string(),
        };
        // persist share info at share_loc
        self.shares_map.insert(uid_pw_hash, perm_share.clone());
        debug!("user share: {:?}", perm_share.share);

        (all_shares[2].clone(), uid_pw_hash)
    }

    pub fn get_priv_key(
        &self,
        uid: &UID,
        password: Option<String>,
        share: Option<(Share, ShareIndex)>,
    ) -> Option<Vec<u8>> {
        let mut shares: Vec<[BigInt; 2]> = vec![];

        if share != None {
            shares.push(share.clone().unwrap().0);

            if let Some(share) = self.shares_map.get(&share.clone().unwrap().1) {
                debug!("getting users share");
                shares.push(share.share.clone());
            }
        } else {
            debug!("using uid and pw");

            if password == None {
                debug!("no password?");
                return None;
            }
            let password_str = password.unwrap();
            let mut uid_owned = uid.clone().to_owned();

            uid_owned.push_str(&password_str);
            let uid_pw_hash = {
                let mut hasher = DefaultHasher::new();
                uid_owned.hash(&mut hasher);
                hasher.finish()
            };
            debug!(
                "got uid, password {} {}: hash {}",
                uid, password_str, uid_pw_hash
            );

            if let Some(share) = self.shares_map.get(&uid_pw_hash) {
                debug!("getting users share");
                shares.push(share.share.clone());

                let pass_info: String = Pbkdf2
                    .hash_password(password_str.as_bytes(), &share.password_salt)
                    .unwrap()
                    .to_string();
                let hash_pass_bigint =
                    BigInt::from_bytes_le(num_bigint::Sign::Plus, pass_info.as_bytes());

                debug!("hash_pass_bigint: {}", hash_pass_bigint);
                debug!("user share: {:?}", share.share);
                let other_share = [hash_pass_bigint, share.share_value.clone()];
                shares.push(other_share);
            }
        }

        if shares.len() != 2 {
            debug!("Unable to reconstruct due to too few shares");
            return None;
        }

        let sss = ShamirSecretSharing {
            threshold: 1,
            share_count: 3,
            prime: self.prime.clone(),
        };
        let priv_key = sss.reconstruct(&shares);
        let pkbytes = get_pk_bytes(priv_key.to_bytes_le().1);
        return Some(pkbytes.to_vec());
    }
}

/*
 * SHAMIR SECRET SHARING STUFF
 */
pub struct ShamirSecretSharing {
    /// Maximum number of shares that can be known without exposing the secret.
    pub threshold: usize,
    /// Number of shares to split the secret into.
    pub share_count: usize,
    /// Prime defining the Zp field in which computation is taking place.
    pub prime: BigInt,
}

impl ShamirSecretSharing {
    /// Minimum number of shares required to reconstruct secret.
    ///
    /// For this scheme this is always `threshold + 1`.
    pub fn reconstruct_limit(&self) -> usize {
        self.threshold + 1
    }

    /// Generate `share_count` shares from `secret`.
    pub fn share(&self, secret: &BigInt, hash_pass: &BigInt) -> Vec<[BigInt; 2]> {
        let result = self.sample_polynomial(secret, hash_pass);
        self.evaluate_polynomial(&result[0], &result[1])
    }

    /// Reconstruct `secret` from a large enough subset of the shares.
    ///
    /// `indices` are the ranks of the known shares as output by the `share` method,
    /// while `values` are the actual values of these shares.
    /// Both must have the same number of elements, and at least `reconstruct_limit`.
    pub fn reconstruct(&self, shares: &Vec<[BigInt; 2]>) -> BigInt {
        assert!(shares.len() >= self.reconstruct_limit());
        // add one to indices to get points
        let x_values: Vec<BigInt> = (0..self.reconstruct_limit())
            .map(|i| shares[i][0].clone())
            .collect();
        let y_values: Vec<BigInt> = (0..self.reconstruct_limit())
            .map(|i| shares[i][1].clone())
            .collect();
        lagrange_interpolation_at_zero(&x_values, &y_values, &self.prime)
    }

    fn sample_polynomial(&self, zero_value: &BigInt, hash_pass: &BigInt) -> [Vec<BigInt>; 2] {
        // fix the first coefficient (corresponding to the evaluation at zero)
        let mut coefficients = vec![zero_value.clone()];
        // sample the remaining coefficients randomly using secure randomness
        let range = Uniform::from(0..i64::MAX);
        let mut rng = rand::thread_rng();
        let random_coefficients: Vec<BigInt> = (0..self.threshold)
            .map(|_| BigInt::from(range.sample(&mut rng)))
            .collect();
        coefficients.extend(random_coefficients);

        let mut points = vec![hash_pass.clone()];
        let random_points: Vec<BigInt> = (0..self.share_count - 1)
            .map(|_| BigInt::from(range.sample(&mut rng)))
            .collect();
        points.extend(random_points);
        // return
        [coefficients, points]
    }

    fn evaluate_polynomial(&self, coefficients: &[BigInt], points: &[BigInt]) -> Vec<[BigInt; 2]> {
        // evaluate at all points
        let mut result: Vec<[BigInt; 2]> = vec![];
        for point in points {
            let y_val = mod_evaluate_polynomial(coefficients, point, &self.prime);
            result.push([point.clone(), y_val.clone()]);
        }
        result
    }
}

pub fn mod_evaluate_polynomial(coefficients: &[BigInt], point: &BigInt, prime: &BigInt) -> BigInt {
    // evaluate using Horner's rule
    //  - to combine with fold we consider the coefficients in reverse order
    let mut reversed_coefficients = coefficients.iter().rev();
    // manually split due to fold insisting on an initial value
    let head = reversed_coefficients.next().unwrap();
    let tail = reversed_coefficients;
    tail.fold(head.clone(), |partial, coef| {
        (partial * point + coef) % prime
    })
}

pub fn gcd(a: &BigInt, b: &BigInt) -> (BigInt, BigInt, BigInt) {
    if *b == BigInt::from(0) {
        (a.clone(), BigInt::from(1), BigInt::from(0))
    } else {
        let n = a.checked_div(b).expect("dividing overflowed");
        let c = a % b;
        let r = gcd(b, &c);
        (r.0, r.2.clone(), r.1 - r.2.clone() * n)
    }
}

/// Inverse of `k` in the *Zp* field defined by `prime`.
pub fn mod_inverse(k: &BigInt, prime: &BigInt) -> BigInt {
    let k2 = k % prime;
    let r = if k2 < BigInt::from(0) {
        -gcd(prime, &-k2).2
    } else {
        gcd(prime, &k2).2
    };
    (prime + r) % prime
}

pub fn lagrange_interpolation_at_zero(
    points: &[BigInt],
    values: &[BigInt],
    prime: &BigInt,
) -> BigInt {
    assert_eq!(points.len(), values.len());
    // Lagrange interpolation for point 0
    let mut acc = BigInt::from(0);
    for i in 0..values.len() {
        let xi = &points[i];
        let yi = &values[i];
        let mut num = BigInt::from(1);
        let mut denum = BigInt::from(1);
        for j in 0..values.len() {
            if j != i {
                let xj = &points[j];
                let num_xj_prod = num.checked_mul(xj).expect("multiplication overflow");
                num = modulus(&num_xj_prod, &prime);
                let xj_xi_diff = xj.checked_sub(xi).expect("subtraction overflow");
                let dnum_diff_prod = denum
                    .checked_mul(&xj_xi_diff)
                    .expect("second multiplication overflow");
                denum = modulus(&dnum_diff_prod, prime);
            }
        }
        let mod_inv = mod_inverse(&denum, &prime);
        let prod1 = yi.checked_mul(&num).expect("multiplication overflow");
        let prod2 = prod1
            .checked_mul(&mod_inv)
            .expect("multiplication overflow");
        let sum = acc.checked_add(&prod2).expect("addition overflow");
        acc = sum % prime;
    }
    acc
}

fn modulus(a: &BigInt, m: &BigInt) -> BigInt {
    ((a % m) + m) % m
}

// tests

#[test]
fn test_lagrange_interpolation_at_zero() {
    let res1 = lagrange_interpolation_at_zero(
        &[BigInt::from(1), BigInt::from(2)],
        &[BigInt::from(4), BigInt::from(5)],
        &BigInt::from(1613),
    );

    assert_eq!(res1, BigInt::from(3))
}

#[test]
fn test_evaluate_polynomial() {
    use num_primes::Generator;
    let secretkey_int = BigInt::from(1234);
    let hash_pass = BigInt::from(5);

    let prime_gen = Generator::new_prime(256).to_bytes_le();
    let prime_arr: [u8; 32] = prime_gen
        .try_into()
        .expect("Could not turn u64 vec into bytes?");
    let prime = BigInt::from_bytes_le(num_bigint::Sign::Plus, &prime_arr);

    let ref tss = ShamirSecretSharing {
        threshold: 1,
        share_count: 3,
        prime: prime,
    };
    let shares = tss.share(&secretkey_int, &hash_pass);

    assert_eq!(tss.reconstruct(&(shares[..2].to_vec())), BigInt::from(1234));
}

#[test]
fn simple_realistic_test() {
    use num_primes::Generator;
    use rand::rngs::OsRng;
    let mut rng = OsRng;

    let secretkey = SecretKey::generate(&mut rng);
    let secretkey_int = BigInt::from_bytes_le(num_bigint::Sign::Plus, secretkey.as_bytes());

    let hash_pass = BigInt::from(524736458);

    let prime_gen = Generator::new_prime(512).to_bytes_le();
    let prime_arr: [u8; 64] = prime_gen
        .try_into()
        .expect("Could not turn u64 vec into bytes?");
    let prime = BigInt::from_bytes_le(num_bigint::Sign::Plus, &prime_arr);

    let sss = ShamirSecretSharing {
        threshold: 1,
        share_count: 3,
        prime: prime,
    };

    let all_shares = sss.share(&secretkey_int, &hash_pass);

    assert_eq!(sss.reconstruct(&(all_shares[..2].to_vec())), secretkey_int);
}

#[test]
fn realistic_test() {
    use num_primes::Generator;
    use pbkdf2::{
        password_hash::{PasswordHasher, SaltString},
        Pbkdf2,
    };
    use rand::rngs::OsRng;

    // --------------- construction ---------------

    let mut rng = OsRng;

    let secretkey = SecretKey::generate(&mut rng);
    let secretkey_int = BigInt::from_bytes_le(num_bigint::Sign::Plus, secretkey.as_bytes());

    let password = String::from("password");

    let salt = SaltString::generate(&mut rng);
    let _pass_info: String = Pbkdf2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    // let hash_pass_bigint = BigInt::from_bytes_le(num_bigint::Sign::Plus, pass_info.as_bytes());
    let hash_pass_bigint = BigInt::from(123456789);

    let prime_gen = Generator::new_prime(512).to_bytes_le();
    let prime_arr: [u8; 64] = prime_gen
        .try_into()
        .expect("Could not turn u64 vec into bytes?");
    let prime = BigInt::from_bytes_le(num_bigint::Sign::Plus, &prime_arr);

    println!("prime: {}", prime.clone());

    let sss = ShamirSecretSharing {
        threshold: 1,
        share_count: 3,
        prime: prime.clone(),
    };

    let all_shares = sss.share(&secretkey_int, &hash_pass_bigint);

    // --------------- reconstruction ---------------

    let mut recon_shares: Vec<[BigInt; 2]> = vec![];

    let edna_share = all_shares[1].clone();
    let edna_share_value = all_shares[0][1].clone();

    let _pass_info2: String = Pbkdf2
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    // let hash_pass_bigint2 = BigInt::from_bytes_le(num_bigint::Sign::Plus, pass_info2.as_bytes());
    let hash_pass_bigint2 = BigInt::from(123456789);

    let derived_share = [hash_pass_bigint2, edna_share_value.clone()];

    recon_shares.push(edna_share);
    recon_shares.push(derived_share);

    println!("share 0: {}, {}", recon_shares[0][0], recon_shares[0][1]);
    println!("share 1: {}, {}", recon_shares[1][0], recon_shares[1][1]);

    // --------------- assert equal ---------------

    println!("checking fake reconstruction");
    let alleged_priv_key = sss.reconstruct(&(all_shares[..2].to_vec()));
    println!("private key: {}", secretkey_int);
    println!("found private key: {}", alleged_priv_key);
    assert!(secretkey_int.eq(&alleged_priv_key));

    let recon_priv_key = sss.reconstruct(&recon_shares);
    println!("checking actual reconstruction");
    println!("private key: {}", secretkey_int);
    println!("found private key: {}", recon_priv_key);
    assert!(secretkey_int.eq(&recon_priv_key));
}
