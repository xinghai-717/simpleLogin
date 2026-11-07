use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

/// RSA 明文格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretFormat {
    Pwd,
    PwdMac,
    PwdIp,
}

impl Default for SecretFormat {
    fn default() -> Self {
        SecretFormat::Pwd
    }
}

/// 拼接 RSA 明文（对应 Python 的 build_secret）
pub fn build_secret(password: &str, format: SecretFormat, mac: &str, ip: &str) -> String {
    match format {
        SecretFormat::Pwd => password.to_string(),
        SecretFormat::PwdMac => format!("{password}>{mac}"),
        SecretFormat::PwdIp => format!("{password}>{ip}"),
    }
}

/// 复刻前端 Tom Wu RSA 的 encryptedPwd 实现
pub fn rsa_encrypt(secret: &str, e_hex: &str, n_hex: &str) -> String {
    let mut raw = secret.as_bytes().to_vec();
    raw.reverse();
    let m = BigUint::from_bytes_be(&raw);
    let e = BigUint::parse_bytes(e_hex.as_bytes(), 16)
        .expect("invalid exponent hex");
    let n = BigUint::parse_bytes(n_hex.as_bytes(), 16)
        .expect("invalid modulus hex");
    let c = m.modpow(&e, &n);
    let mod_bytes = (n_hex.len() + 1) / 2;
    format!("{:0>width$x}", c, width = mod_bytes * 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 一组固定的"伪参数"——不需要真实 RSA 语义，只验证两边算法一致
    const FAKE_E: &str = "10001";

    // 短 n：m < n 容易满足，快速验证基本逻辑
    const FAKE_N_SHORT: &str = "c0ffee";

    // 512 字符：模拟真实 2048 位模数，验证左补零逻辑
    const FAKE_N_LONG: &str = "4a4704e0ddcad488084e84886b83615a9f1551cb6a7c29313f2e7958c84c6bfa596d635be8087d6d626314f0734b58637dc48332bec3288ef7f069b4b87e41af4f29baa20c133fe9ad1b82c238f030acfcfd05260b84be5d55c8acfd6869262c5a92f4cc4e385c61b337063a9f9f7ca332e3efdeadee93aa0099deb9bac82d48ec3e34c3ffc835085cf4fb782861eac20f090cc813da6a4957a02619ce25a61a640ce51d675847398268b626a69ac164cb8089106f86ddfbf9145b92234df2d9f465bf4744c7e4dd7d02174e897d376e106af6476f9be30228f0e4b6064647ea50ce0d3437c603142663eec919bc9b03d3c7021ec11767c271862535277140f9";

    #[test]
    fn rsa_short_n() {
        // Python: rsa_encrypt("test", "10001", "c0ffee")
        // 把 Python 输出粘贴到 expected
        let got = rsa_encrypt("test", FAKE_E, FAKE_N_SHORT);
        assert_eq!(got, "5fd136");
    }

    #[test]
    fn rsa_long_n() {
        let got = rsa_encrypt("test", FAKE_E, FAKE_N_LONG);
        assert_eq!(got, "123579b2fa280c8d3212683b63a62a81d5d8b57ae649549180dfd60c0f7049a12177c7a2bc63f2954101ddeef0b799dc90490b6e4bf4214c794e489a890b373701c5952570621918b146ec86e123cf37dd350586d8a49f364492443d567cb38aaea1ee5ad799f3ee565545699f3cbf90ae3ce2add5dc239633a319d3ead8a32d3f6044754f850eff588a9e4c6f535c08570cdf5924276a34a17468d66052909af26484e1f725fc254b4bea1b2af42f6f8fc6f949bcfbe270e2ba9bca32c1ddc02c0a5945ebdbbf4b0c075de503c28fe6a2990f5c68bc3e5f44cf7650a4aaad06a1faab987bd0b15e4d86cf899b2cc9e1d127caeb86ae10377d4a210bea4cd2bf");
    }

    #[test]
    fn rsa_chinese_secret() {
        // 验证 UTF-8 编码路径：中文密码
        let got = rsa_encrypt("密码123", FAKE_E, FAKE_N_LONG);
        assert_eq!(got, "165de332411b7e36415704fb1c5d0772d409e21aa968beccf723c0a7d0b67d1d959f7750938b010a268abc7b25217276bfabf69c051bd51a453d379af3c789ddb488ef388fe1b1fb98c93743925462e10528fce5c141a3352e8116a2370d7c37dd35f68503d29420e8ad4ba2a339134d1361518fcbb96f565f8be7b92e82395500792a1fe0ef5e2c05a2445d9c649f4b9fd3607bfa7f880677e3ed166412c9d8d409518736439c30ac9464c78978c1871b2843caae457d5345e88bb0d099fb77dbbd5da162a00f0b3f27528577244bc4345ec8412a01eb94d73e9f18325371669fdf752f6c4cf7b08867f83a6f4909838c18e18620c5d1f674379b598e703428");
    }

    #[test]
    fn rsa_length_is_fixed() {
        // 验证补零：输出长度必须等于 (n_hex.len()+1)/2*2
        let out = rsa_encrypt("test", FAKE_E, FAKE_N_LONG);
        let expected_len = (FAKE_N_LONG.len() + 1) / 2 * 2;
        assert_eq!(out.len(), expected_len, "补零逻辑有问题");
    }
}