use super::*;
use crate::object::Object;
use std::collections::HashMap;

#[test]
fn test_algorithm_key_length_none() {
    assert_eq!(Algorithm::None.key_length(), 0);
}

#[test]
fn test_algorithm_key_length_rc4_40() {
    assert_eq!(Algorithm::RC4_40.key_length(), 5);
}

#[test]
fn test_algorithm_key_length_rc4_128() {
    assert_eq!(Algorithm::Rc4_128.key_length(), 16);
}

#[test]
fn test_algorithm_key_length_aes128() {
    assert_eq!(Algorithm::Aes128.key_length(), 16);
}

#[test]
fn test_algorithm_key_length_aes256() {
    assert_eq!(Algorithm::Aes256.key_length(), 32);
}

#[test]
fn test_algorithm_is_aes() {
    assert!(!Algorithm::None.is_aes());
    assert!(!Algorithm::RC4_40.is_aes());
    assert!(!Algorithm::Rc4_128.is_aes());
    assert!(Algorithm::Aes128.is_aes());
    assert!(Algorithm::Aes256.is_aes());
}

#[test]
fn test_algorithm_is_rc4() {
    assert!(!Algorithm::None.is_rc4());
    assert!(Algorithm::RC4_40.is_rc4());
    assert!(Algorithm::Rc4_128.is_rc4());
    assert!(!Algorithm::Aes128.is_rc4());
    assert!(!Algorithm::Aes256.is_rc4());
}

#[test]
fn test_algorithm_none_not_aes_not_rc4() {
    assert!(!Algorithm::None.is_aes());
    assert!(!Algorithm::None.is_rc4());
}

#[test]
fn test_algorithm_equality() {
    assert_eq!(Algorithm::RC4_40, Algorithm::RC4_40);
    assert_eq!(Algorithm::Aes256, Algorithm::Aes256);
    assert_ne!(Algorithm::RC4_40, Algorithm::Rc4_128);
    assert_ne!(Algorithm::Aes128, Algorithm::Aes256);
}

#[test]
fn test_algorithm_clone_copy() {
    let a = Algorithm::Aes128;
    let b = a;
    let c = a;
    assert_eq!(a, b);
    assert_eq!(a, c);
}

fn make_encrypt_dict_obj(filter: &str, v: i64, r: i64, owner: &[u8], user: &[u8], p: i64) -> Object {
    let mut dict = HashMap::new();
    dict.insert("Filter".to_string(), Object::Name(filter.to_string()));
    dict.insert("V".to_string(), Object::Integer(v));
    dict.insert("R".to_string(), Object::Integer(r));
    dict.insert("O".to_string(), Object::String(owner.to_vec()));
    dict.insert("U".to_string(), Object::String(user.to_vec()));
    dict.insert("P".to_string(), Object::Integer(p));
    Object::Dictionary(dict)
}

#[test]
fn test_encrypt_dict_from_object_rc4_40() {
    let obj = make_encrypt_dict_obj("Standard", 1, 2, &[0u8; 32], &[0u8; 32], -4);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.filter, "Standard");
    assert_eq!(ed.version, 1);
    assert_eq!(ed.revision, 2);
    assert_eq!(ed.permissions, -4);
    assert!(ed.encrypt_metadata);
    assert!(ed.sub_filter.is_none());
    assert!(ed.length.is_none());
}

#[test]
fn test_encrypt_dict_from_object_rc4_128() {
    let obj = make_encrypt_dict_obj("Standard", 2, 3, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.version, 2);
    assert_eq!(ed.revision, 3);
}

#[test]
fn test_encrypt_dict_from_object_aes128() {
    let obj = make_encrypt_dict_obj("Standard", 4, 4, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.version, 4);
    assert_eq!(ed.revision, 4);
}

#[test]
fn test_encrypt_dict_from_object_aes256() {
    let mut dict = HashMap::new();
    dict.insert("Filter".to_string(), Object::Name("Standard".to_string()));
    dict.insert("V".to_string(), Object::Integer(5));
    dict.insert("R".to_string(), Object::Integer(6));
    dict.insert("O".to_string(), Object::String(vec![0u8; 48]));
    dict.insert("U".to_string(), Object::String(vec![0u8; 48]));
    dict.insert("P".to_string(), Object::Integer(-1));
    dict.insert("OE".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("UE".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("Perms".to_string(), Object::String(vec![0u8; 16]));
    let obj = Object::Dictionary(dict);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.version, 5);
    assert_eq!(ed.revision, 6);
    assert!(ed.owner_encryption.is_some());
    assert!(ed.user_encryption.is_some());
    assert!(ed.perms.is_some());
}

#[test]
fn test_encrypt_dict_from_object_with_optional_fields() {
    let mut dict = HashMap::new();
    dict.insert("Filter".to_string(), Object::Name("Standard".to_string()));
    dict.insert("SubFilter".to_string(), Object::Name("adbe.pkcs7.s4".to_string()));
    dict.insert("V".to_string(), Object::Integer(2));
    dict.insert("R".to_string(), Object::Integer(3));
    dict.insert("Length".to_string(), Object::Integer(128));
    dict.insert("O".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("U".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("P".to_string(), Object::Integer(-3904));
    dict.insert("EncryptMetadata".to_string(), Object::Boolean(false));
    let obj = Object::Dictionary(dict);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.sub_filter.as_deref(), Some("adbe.pkcs7.s4"));
    assert_eq!(ed.length, Some(128));
    assert!(!ed.encrypt_metadata);
}

#[test]
fn test_encrypt_dict_from_object_not_dict() {
    let obj = Object::Integer(42);
    let result = EncryptDict::from_object(&obj);
    assert!(result.is_err());
}

#[test]
fn test_encrypt_dict_from_object_missing_filter() {
    let mut dict = HashMap::new();
    dict.insert("V".to_string(), Object::Integer(1));
    dict.insert("R".to_string(), Object::Integer(2));
    dict.insert("O".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("U".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("P".to_string(), Object::Integer(-1));
    let obj = Object::Dictionary(dict);
    let result = EncryptDict::from_object(&obj);
    assert!(result.is_err());
}

#[test]
fn test_encrypt_dict_from_object_missing_v() {
    let mut dict = HashMap::new();
    dict.insert("Filter".to_string(), Object::Name("Standard".to_string()));
    dict.insert("R".to_string(), Object::Integer(2));
    dict.insert("O".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("U".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("P".to_string(), Object::Integer(-1));
    let obj = Object::Dictionary(dict);
    let result = EncryptDict::from_object(&obj);
    assert!(result.is_err());
}

#[test]
fn test_encrypt_dict_from_object_missing_r() {
    let mut dict = HashMap::new();
    dict.insert("Filter".to_string(), Object::Name("Standard".to_string()));
    dict.insert("V".to_string(), Object::Integer(1));
    dict.insert("O".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("U".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("P".to_string(), Object::Integer(-1));
    let obj = Object::Dictionary(dict);
    let result = EncryptDict::from_object(&obj);
    assert!(result.is_err());
}

#[test]
fn test_encrypt_dict_from_object_missing_o() {
    let mut dict = HashMap::new();
    dict.insert("Filter".to_string(), Object::Name("Standard".to_string()));
    dict.insert("V".to_string(), Object::Integer(1));
    dict.insert("R".to_string(), Object::Integer(2));
    dict.insert("U".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("P".to_string(), Object::Integer(-1));
    let obj = Object::Dictionary(dict);
    let result = EncryptDict::from_object(&obj);
    assert!(result.is_err());
}

#[test]
fn test_encrypt_dict_from_object_missing_u() {
    let mut dict = HashMap::new();
    dict.insert("Filter".to_string(), Object::Name("Standard".to_string()));
    dict.insert("V".to_string(), Object::Integer(1));
    dict.insert("R".to_string(), Object::Integer(2));
    dict.insert("O".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("P".to_string(), Object::Integer(-1));
    let obj = Object::Dictionary(dict);
    let result = EncryptDict::from_object(&obj);
    assert!(result.is_err());
}

#[test]
fn test_encrypt_dict_from_object_missing_p() {
    let mut dict = HashMap::new();
    dict.insert("Filter".to_string(), Object::Name("Standard".to_string()));
    dict.insert("V".to_string(), Object::Integer(1));
    dict.insert("R".to_string(), Object::Integer(2));
    dict.insert("O".to_string(), Object::String(vec![0u8; 32]));
    dict.insert("U".to_string(), Object::String(vec![0u8; 32]));
    let obj = Object::Dictionary(dict);
    let result = EncryptDict::from_object(&obj);
    assert!(result.is_err());
}

#[test]
fn test_algorithm_detection_v1_r2() {
    let obj = make_encrypt_dict_obj("Standard", 1, 2, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.algorithm().unwrap(), Algorithm::RC4_40);
}

#[test]
fn test_algorithm_detection_v2_r3() {
    let obj = make_encrypt_dict_obj("Standard", 2, 3, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.algorithm().unwrap(), Algorithm::Rc4_128);
}

#[test]
fn test_algorithm_detection_lenient_v1_r3() {
    // Non-standard R for V=1 should still return RC4_40 ~keep
    let obj = make_encrypt_dict_obj("Standard", 1, 3, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.algorithm().unwrap(), Algorithm::RC4_40);
}

#[test]
fn test_algorithm_detection_lenient_v2_r4() {
    // Non-standard R for V=2 should still return Rc4_128 ~keep
    let obj = make_encrypt_dict_obj("Standard", 2, 4, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.algorithm().unwrap(), Algorithm::Rc4_128);
}

#[derive(Clone, Copy)]
enum FilterEntry {
    Absent,
    Null,
    Named,
    Identity,
}

struct CryptFilterCase {
    name: &'static str,
    version: i64,
    revision: i64,
    lenient: bool,
    stm_f: FilterEntry,
    str_f: FilterEntry,
    stream: Algorithm,
    string: Algorithm,
}

fn crypt_filter_dict_obj(case: &CryptFilterCase) -> Object {
    let Object::Dictionary(mut dict) =
        make_encrypt_dict_obj("Standard", case.version, case.revision, &[0u8; 32], &[0u8; 32], -1)
    else {
        unreachable!()
    };
    let cfm = if case.version == 5 { "AESV3" } else { "AESV2" };
    let mut std_cf = HashMap::new();
    std_cf.insert("CFM".to_string(), Object::Name(cfm.to_string()));
    let mut cf = HashMap::new();
    cf.insert("StdCF".to_string(), Object::Dictionary(std_cf));
    dict.insert("CF".to_string(), Object::Dictionary(cf));
    for (key, entry) in [("StmF", case.stm_f), ("StrF", case.str_f)] {
        match entry {
            FilterEntry::Absent => {}
            FilterEntry::Null => {
                dict.insert(key.to_string(), Object::Null);
            }
            FilterEntry::Named => {
                dict.insert(key.to_string(), Object::Name("StdCF".to_string()));
            }
            FilterEntry::Identity => {
                dict.insert(key.to_string(), Object::Name("Identity".to_string()));
            }
        }
    }
    Object::Dictionary(dict)
}

#[test]
fn test_algorithm_detection_v4_v5_crypt_filter_defaults() {
    use Algorithm::{Aes128, Aes256, None as Plain};
    use FilterEntry::{Absent, Identity, Named, Null};
    let case = |name, version, revision, lenient, stm_f, str_f, stream, string| CryptFilterCase {
        name,
        version,
        revision,
        lenient,
        stm_f,
        str_f,
        stream,
        string,
    };
    let cases = [
        case("v4_r4_both_absent", 4, 4, false, Absent, Absent, Plain, Plain),
        case("v5_r5_both_absent", 5, 5, false, Absent, Absent, Plain, Plain),
        case("v5_r6_both_absent", 5, 6, false, Absent, Absent, Plain, Plain),
        case("lenient_v4_r5_both_absent", 4, 5, true, Absent, Absent, Plain, Plain),
        case("v4_r4_both_null", 4, 4, false, Null, Null, Plain, Plain),
        case("v4_r4_only_strf_present", 4, 4, false, Absent, Named, Plain, Aes128),
        case("v4_r4_only_stmf_present", 4, 4, false, Named, Absent, Aes128, Plain),
        case("v5_r6_only_strf_present", 5, 6, false, Absent, Named, Plain, Aes256),
        case("v5_r6_only_stmf_present", 5, 6, false, Named, Absent, Aes256, Plain),
        case(
            "lenient_v4_r5_only_stmf_present",
            4,
            5,
            true,
            Named,
            Absent,
            Aes128,
            Plain,
        ),
        case("v4_r4_both_named", 4, 4, false, Named, Named, Aes128, Aes128),
        case("v5_r5_both_named", 5, 5, false, Named, Named, Aes256, Aes256),
        case("v5_r6_both_named", 5, 6, false, Named, Named, Aes256, Aes256),
        case("lenient_v4_r5_both_named", 4, 5, true, Named, Named, Aes128, Aes128),
        case(
            "v4_r4_explicit_identity_both",
            4,
            4,
            false,
            Identity,
            Identity,
            Plain,
            Plain,
        ),
        case(
            "v5_r6_explicit_identity_stmf_named_strf",
            5,
            6,
            false,
            Identity,
            Named,
            Plain,
            Aes256,
        ),
        case(
            "v4_r4_named_stmf_explicit_identity_strf",
            4,
            4,
            false,
            Named,
            Identity,
            Aes128,
            Plain,
        ),
    ];
    assert_eq!(cases.len(), 17, "case table was truncated");

    for c in &cases {
        let standard = matches!((c.version, c.revision), (4, 4) | (5, 5) | (5, 6));
        assert_eq!(c.lenient, !standard, "{}: lenient flag disagrees with (V, R)", c.name);
        let ed = EncryptDict::from_object(&crypt_filter_dict_obj(c)).unwrap();
        assert_eq!(ed.algorithm().unwrap(), c.stream, "{}: stream algorithm", c.name);
        assert_eq!(ed.string_algorithm().unwrap(), c.string, "{}: string algorithm", c.name);
    }
}

#[test]
fn test_algorithm_detection_unsupported() {
    let obj = make_encrypt_dict_obj("Standard", 99, 99, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    let result = ed.algorithm();
    assert!(result.is_err());
}

#[test]
fn test_key_length_bytes_with_length() {
    let obj = make_encrypt_dict_obj("Standard", 2, 3, &[0u8; 32], &[0u8; 32], -1);
    let mut ed = EncryptDict::from_object(&obj).unwrap();
    ed.length = Some(128);
    assert_eq!(ed.key_length_bytes(), 16);
}

#[test]
fn test_key_length_bytes_with_length_40() {
    let obj = make_encrypt_dict_obj("Standard", 1, 2, &[0u8; 32], &[0u8; 32], -1);
    let mut ed = EncryptDict::from_object(&obj).unwrap();
    ed.length = Some(40);
    assert_eq!(ed.key_length_bytes(), 5);
}

#[test]
fn test_key_length_bytes_default_v1() {
    let obj = make_encrypt_dict_obj("Standard", 1, 2, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.key_length_bytes(), 5);
}

#[test]
fn test_key_length_bytes_default_v2() {
    let obj = make_encrypt_dict_obj("Standard", 2, 3, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.key_length_bytes(), 16);
}

#[test]
fn test_key_length_bytes_default_v4() {
    let obj = make_encrypt_dict_obj("Standard", 4, 4, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.key_length_bytes(), 16);
}

#[test]
fn test_key_length_bytes_default_v5() {
    let obj = make_encrypt_dict_obj("Standard", 5, 6, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.key_length_bytes(), 32);
}

#[test]
fn test_key_length_bytes_default_unknown_v() {
    let obj = make_encrypt_dict_obj("Standard", 3, 3, &[0u8; 32], &[0u8; 32], -1);
    let ed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(ed.key_length_bytes(), 16);
}

#[test]
fn test_to_object_basic() {
    let ed = EncryptDict {
        filter: "Standard".to_string(),
        sub_filter: None,
        version: 1,
        length: None,
        revision: 2,
        owner_password: vec![0u8; 32],
        user_password: vec![0u8; 32],
        permissions: -4,
        encrypt_metadata: true,
        owner_encryption: None,
        user_encryption: None,
        perms: None,
        stream_crypt_method: None,
        string_crypt_method: None,
    };
    let obj = ed.to_object();
    let dict = obj.as_dict().unwrap();
    assert!(dict.contains_key("Filter"));
    assert!(dict.contains_key("V"));
    assert!(dict.contains_key("R"));
    assert!(dict.contains_key("O"));
    assert!(dict.contains_key("U"));
    assert!(dict.contains_key("P"));
    // EncryptMetadata is true (default), should NOT be written ~keep
    assert!(!dict.contains_key("EncryptMetadata"));
}

#[test]
fn test_to_object_with_sub_filter() {
    let ed = EncryptDict {
        filter: "Standard".to_string(),
        sub_filter: Some("adbe.pkcs7.s4".to_string()),
        version: 2,
        length: Some(128),
        revision: 3,
        owner_password: vec![0u8; 32],
        user_password: vec![0u8; 32],
        permissions: -1,
        encrypt_metadata: false,
        owner_encryption: None,
        user_encryption: None,
        perms: None,
        stream_crypt_method: None,
        string_crypt_method: None,
    };
    let obj = ed.to_object();
    let dict = obj.as_dict().unwrap();
    assert!(dict.contains_key("SubFilter"));
    assert!(dict.contains_key("Length"));
    assert!(dict.contains_key("EncryptMetadata"));
}

#[test]
fn test_to_object_v4_has_crypt_filters() {
    let ed = EncryptDict {
        filter: "Standard".to_string(),
        sub_filter: None,
        version: 4,
        length: None,
        revision: 4,
        owner_password: vec![0u8; 32],
        user_password: vec![0u8; 32],
        permissions: -1,
        encrypt_metadata: true,
        owner_encryption: None,
        user_encryption: None,
        perms: None,
        stream_crypt_method: None,
        string_crypt_method: None,
    };
    let obj = ed.to_object();
    let dict = obj.as_dict().unwrap();
    assert!(dict.contains_key("CF"));
    assert!(dict.contains_key("StmF"));
    assert!(dict.contains_key("StrF"));
}

#[test]
fn test_to_object_v5_has_crypt_filters() {
    let ed = EncryptDict {
        filter: "Standard".to_string(),
        sub_filter: None,
        version: 5,
        length: None,
        revision: 6,
        owner_password: vec![0u8; 48],
        user_password: vec![0u8; 48],
        permissions: -1,
        encrypt_metadata: true,
        owner_encryption: Some(vec![0u8; 32]),
        user_encryption: Some(vec![0u8; 32]),
        perms: Some(vec![0u8; 16]),
        stream_crypt_method: None,
        string_crypt_method: None,
    };
    let obj = ed.to_object();
    let dict = obj.as_dict().unwrap();
    assert!(dict.contains_key("CF"));
    assert!(dict.contains_key("StmF"));
    assert!(dict.contains_key("StrF"));
    assert!(dict.contains_key("OE"));
    assert!(dict.contains_key("UE"));
    assert!(dict.contains_key("Perms"));
}

#[test]
fn test_permissions_all_granted() {
    let perms = Permissions::from_bits(-1i32);
    assert!(perms.can_print());
    assert!(perms.can_modify());
    assert!(perms.can_copy());
    assert!(perms.can_annotate());
    assert!(perms.can_fill_forms());
    assert!(perms.can_extract_accessibility());
    assert!(perms.can_assemble());
    assert!(perms.can_print_high_quality());
}

#[test]
fn test_permissions_none_granted() {
    let perms = Permissions::from_bits(0);
    assert!(!perms.can_print());
    assert!(!perms.can_modify());
    assert!(!perms.can_copy());
    assert!(!perms.can_annotate());
    assert!(!perms.can_fill_forms());
    assert!(!perms.can_extract_accessibility());
    assert!(!perms.can_assemble());
    assert!(!perms.can_print_high_quality());
}

#[test]
fn test_permissions_print_only() {
    let perms = Permissions::from_bits(1 << 2);
    assert!(perms.can_print());
    assert!(!perms.can_modify());
    assert!(!perms.can_copy());
}

#[test]
fn test_permissions_modify_only() {
    let perms = Permissions::from_bits(1 << 3);
    assert!(!perms.can_print());
    assert!(perms.can_modify());
    assert!(!perms.can_copy());
}

#[test]
fn test_permissions_copy_only() {
    let perms = Permissions::from_bits(1 << 4);
    assert!(perms.can_copy());
    assert!(!perms.can_print());
    assert!(!perms.can_modify());
}

#[test]
fn test_permissions_annotate_only() {
    let perms = Permissions::from_bits(1 << 5);
    assert!(perms.can_annotate());
}

#[test]
fn test_permissions_fill_forms_only() {
    let perms = Permissions::from_bits(1 << 8);
    assert!(perms.can_fill_forms());
}

#[test]
fn test_permissions_extract_accessibility_only() {
    let perms = Permissions::from_bits(1 << 9);
    assert!(perms.can_extract_accessibility());
}

#[test]
fn test_permissions_assemble_only() {
    let perms = Permissions::from_bits(1 << 10);
    assert!(perms.can_assemble());
}

#[test]
fn test_permissions_high_quality_print_only() {
    let perms = Permissions::from_bits(1 << 11);
    assert!(perms.can_print_high_quality());
}

#[test]
fn test_permissions_combined() {
    // Print + Copy + Fill Forms ~keep
    let bits = (1 << 2) | (1 << 4) | (1 << 8);
    let perms = Permissions::from_bits(bits);
    assert!(perms.can_print());
    assert!(!perms.can_modify());
    assert!(perms.can_copy());
    assert!(!perms.can_annotate());
    assert!(perms.can_fill_forms());
}

#[test]
fn test_permissions_clone_copy() {
    let p = Permissions::from_bits(-1);
    let p2 = p;
    let p3 = p;
    assert!(p2.can_print());
    assert!(p3.can_print());
}

#[test]
fn test_builder_rc4_40() {
    let file_id = vec![0u8; 16];
    let ed = EncryptDictBuilder::new(Algorithm::RC4_40)
        .user_password(b"user")
        .owner_password(b"owner")
        .permissions(-4)
        .build(&file_id)
        .unwrap();
    assert_eq!(ed.filter, "Standard");
    assert_eq!(ed.version, 1);
    assert_eq!(ed.revision, 2);
    assert_eq!(ed.permissions, -4);
    assert_eq!(ed.length, Some(40));
    assert!(ed.encrypt_metadata);
}

#[test]
fn test_builder_rc4_128() {
    let file_id = vec![0u8; 16];
    let ed = EncryptDictBuilder::new(Algorithm::Rc4_128)
        .user_password(b"pass")
        .build(&file_id)
        .unwrap();
    assert_eq!(ed.version, 2);
    assert_eq!(ed.revision, 3);
    assert_eq!(ed.length, Some(128));
}

#[test]
fn test_builder_aes128() {
    let file_id = vec![0u8; 16];
    let ed = EncryptDictBuilder::new(Algorithm::Aes128)
        .user_password(b"pass")
        .encrypt_metadata(false)
        .build(&file_id)
        .unwrap();
    assert_eq!(ed.version, 4);
    assert_eq!(ed.revision, 4);
    assert!(!ed.encrypt_metadata);
}

#[test]
fn test_builder_aes256() {
    let file_id = vec![0u8; 16];
    let ed = EncryptDictBuilder::new(Algorithm::Aes256)
        .user_password(b"user")
        .owner_password(b"owner")
        .build(&file_id)
        .unwrap();
    assert_eq!(ed.version, 5);
    assert_eq!(ed.revision, 6);
    assert_eq!(ed.length, Some(256));
}

#[test]
fn test_builder_owner_password_defaults_to_user() {
    let file_id = vec![0u8; 16];
    let ed = EncryptDictBuilder::new(Algorithm::RC4_40)
        .user_password(b"user_pass")
        .build(&file_id)
        .unwrap();
    assert_eq!(ed.filter, "Standard");
    assert!(!ed.owner_password.is_empty());
}

#[test]
fn test_builder_default_permissions() {
    let file_id = vec![0u8; 16];
    let ed = EncryptDictBuilder::new(Algorithm::RC4_40)
        .user_password(b"pass")
        .build(&file_id)
        .unwrap();
    assert_eq!(ed.permissions, -1);
}

#[test]
fn test_generate_file_id_returns_16_bytes() {
    let (id1, id2) = generate_file_id();
    assert_eq!(id1.len(), 16);
    assert_eq!(id2.len(), 16);
}

#[test]
fn test_generate_file_id_both_equal() {
    let (id1, id2) = generate_file_id();
    assert_eq!(id1, id2);
}

#[test]
fn test_generate_file_id_unique() {
    let (id1, _) = generate_file_id();
    let (id2, _) = generate_file_id();
    // It's extremely unlikely these would be equal ~keep
    assert_ne!(id1, id2);
}

#[test]
fn test_generate_file_id_uniqueness() {
    let id1 = generate_file_id();
    let id2 = generate_file_id();
    assert_ne!(id1, id2, "file IDs must be unique across calls");
}

#[test]
fn test_encrypt_dict_roundtrip_v1() {
    let original = EncryptDict {
        filter: "Standard".to_string(),
        sub_filter: None,
        version: 1,
        length: Some(40),
        revision: 2,
        owner_password: vec![1u8; 32],
        user_password: vec![2u8; 32],
        permissions: -4,
        encrypt_metadata: true,
        owner_encryption: None,
        user_encryption: None,
        perms: None,
        stream_crypt_method: None,
        string_crypt_method: None,
    };
    let obj = original.to_object();
    let parsed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(parsed.filter, original.filter);
    assert_eq!(parsed.version, original.version);
    assert_eq!(parsed.revision, original.revision);
    assert_eq!(parsed.permissions, original.permissions);
    assert_eq!(parsed.owner_password, original.owner_password);
    assert_eq!(parsed.user_password, original.user_password);
}

#[test]
fn test_encrypt_dict_roundtrip_v4() {
    let original = EncryptDict {
        filter: "Standard".to_string(),
        sub_filter: None,
        version: 4,
        length: None,
        revision: 4,
        owner_password: vec![3u8; 32],
        user_password: vec![4u8; 32],
        permissions: -1,
        encrypt_metadata: true,
        owner_encryption: None,
        user_encryption: None,
        perms: None,
        stream_crypt_method: None,
        string_crypt_method: None,
    };
    let obj = original.to_object();
    let parsed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(parsed.version, 4);
    assert_eq!(parsed.revision, 4);
    assert_eq!(parsed.algorithm().unwrap(), Algorithm::Aes128);
    assert_eq!(parsed.stream_crypt_method.as_deref(), Some("AESV2"));
}

#[test]
fn test_v4_cfm_v2_selects_rc4_128() {
    // V=4 with CFM=V2 means RC4-128, NOT AES-128.
    // This is the exact case from issue #202 (OpenPDF 1.3.26). ~keep
    let ed = EncryptDict {
        filter: "Standard".to_string(),
        sub_filter: None,
        version: 4,
        length: Some(128),
        revision: 4,
        owner_password: vec![0u8; 32],
        user_password: vec![0u8; 32],
        permissions: -1580,
        encrypt_metadata: false,
        owner_encryption: None,
        user_encryption: None,
        perms: None,
        stream_crypt_method: Some("V2".to_string()),
        string_crypt_method: None,
    };
    assert_eq!(ed.algorithm().unwrap(), Algorithm::Rc4_128);
}

#[test]
fn test_v4_cfm_v2_roundtrip() {
    let original = EncryptDict {
        filter: "Standard".to_string(),
        sub_filter: None,
        version: 4,
        length: Some(128),
        revision: 4,
        owner_password: vec![5u8; 32],
        user_password: vec![6u8; 32],
        permissions: -1580,
        encrypt_metadata: false,
        owner_encryption: None,
        user_encryption: None,
        perms: None,
        stream_crypt_method: Some("V2".to_string()),
        string_crypt_method: None,
    };
    let obj = original.to_object();
    let parsed = EncryptDict::from_object(&obj).unwrap();
    assert_eq!(parsed.version, 4);
    assert_eq!(parsed.stream_crypt_method.as_deref(), Some("V2"));
    assert_eq!(parsed.algorithm().unwrap(), Algorithm::Rc4_128);
}

mod crypt_filters {
    use super::*;
    use crate::document::PdfDocument;
    use crate::object::ObjectRef;
    use md5::{Digest, Md5};

    const FILE_ID: &[u8] = b"0123456789abcdef";
    const PERMISSIONS: i32 = -4;
    const SECRET_TITLE: &[u8] = b"Secret Title";
    const ENCRYPTED_XMP: &[u8] = b"<x:xmpmeta><rdf:RDF><rdf:Description xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\" \
        pdf:Producer=\"Hidden Producer\"/></rdf:RDF></x:xmpmeta>";
    const PLAIN_METADATA: &[u8] = b"<x:xmpmeta>plain packet</x:xmpmeta>";

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02X}")).collect()
    }

    fn aes128_string(key: &[u8], obj_num: u32, plaintext: &[u8]) -> Vec<u8> {
        let mut hasher = Md5::new();
        hasher.update(key);
        hasher.update(&obj_num.to_le_bytes()[..3]);
        hasher.update([0u8, 0u8]);
        hasher.update(b"sAlT");
        let object_key = hasher.finalize();
        let iv = [7u8; 16];
        let mut out = iv.to_vec();
        out.extend(aes::aes128_encrypt(&object_key[..16], &iv, plaintext).unwrap());
        out
    }

    fn encrypted_pdf(stm_f: &str) -> Vec<u8> {
        encrypted_pdf_with_metadata(stm_f, false)
    }

    const IDENTITY_CRYPT_XMP_DICT: &str =
        "/Type /Metadata /Subtype /XML /Filter [/Crypt] /DecodeParms [<< /Name /Identity >>]";

    fn encrypted_pdf_with_metadata(stm_f: &str, encrypt_xmp: bool) -> Vec<u8> {
        let xmp_dict = (!encrypt_xmp).then_some(IDENTITY_CRYPT_XMP_DICT);
        encrypted_pdf_custom(Some(stm_f), xmp_dict, None)
    }

    fn encrypted_pdf_custom(stm_f: Option<&str>, plain_xmp_dict: Option<&str>, extra_object: Option<&str>) -> Vec<u8> {
        let encrypt_xmp = plain_xmp_dict.is_none();
        let stm_f = stm_f.map(|name| format!("/StmF /{name} ")).unwrap_or_default();
        let owner = algorithms::compute_owner_password_hash(b"owner", b"", 4, 16).unwrap();
        let key = algorithms::compute_encryption_key(b"", &owner, PERMISSIONS, FILE_ID, 4, 16, true).unwrap();
        let user = algorithms::compute_user_password_hash(&key, FILE_ID, 4).unwrap();
        let title = aes128_string(&key, 5, SECRET_TITLE);
        let content = b"BT /F1 12 Tf 10 10 Td (Plain body) Tj ET";

        let mut pdf = b"%PDF-1.6\n".to_vec();
        let mut offsets = Vec::new();
        let mut push = |pdf: &mut Vec<u8>, body: Vec<u8>| {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
            pdf.extend_from_slice(&body);
            pdf.extend_from_slice(b"\nendobj\n");
        };
        let stream = |dict: &str, data: &[u8]| {
            let mut body = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
            body.extend_from_slice(data);
            body.extend_from_slice(b"\nendstream");
            body
        };
        push(&mut pdf, b"<< /Type /Catalog /Pages 2 0 R /Metadata 6 0 R >>".to_vec());
        push(&mut pdf, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
        push(
            &mut pdf,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R \
              /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>"
                .to_vec(),
        );
        push(&mut pdf, stream("", content));
        push(&mut pdf, format!("<< /Title <{}> >>", hex(&title)).into_bytes());
        if encrypt_xmp {
            push(
                &mut pdf,
                stream("/Type /Metadata /Subtype /XML", &aes128_string(&key, 6, ENCRYPTED_XMP)),
            );
        } else {
            push(&mut pdf, stream(plain_xmp_dict.unwrap(), PLAIN_METADATA));
        }
        push(
            &mut pdf,
            format!(
                "<< /Filter /Standard /V 4 /R 4 /Length 128 /P {PERMISSIONS} /O <{}> /U <{}> \
                 /CF << /StdCF << /CFM /AESV2 /AuthEvent /DocOpen /Length 16 >> >> \
                 {stm_f}/StrF /StdCF >>",
                hex(&owner),
                hex(&user)
            )
            .into_bytes(),
        );
        if let Some(extra) = extra_object {
            push(&mut pdf, extra.as_bytes().to_vec());
        }

        let xref = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
        for offset in &offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R /Encrypt 7 0 R /ID [<{id}> <{id}>] >>\n\
                 startxref\n{xref}\n%%EOF\n",
                offsets.len() + 1,
                id = hex(FILE_ID)
            )
            .as_bytes(),
        );
        pdf
    }

    #[test]
    fn identity_stream_filter_with_aes_string_filter_reads_plain_streams_and_decrypts_strings() {
        let doc = PdfDocument::from_bytes(encrypted_pdf("Identity")).unwrap();

        assert!(doc.extract_text(0).unwrap().contains("Plain body"));
        let info = doc.load_object(ObjectRef::new(5, 0)).unwrap();
        assert_eq!(
            info.as_dict().and_then(|d| d.get("Title")),
            Some(&Object::String(SECRET_TITLE.to_vec()))
        );
    }

    #[test]
    fn crypt_filter_identity_stream_bypasses_document_decryption() {
        let doc = PdfDocument::from_bytes(encrypted_pdf("StdCF")).unwrap();
        let metadata_ref = ObjectRef::new(6, 0);
        let metadata = doc.load_object(metadata_ref).unwrap();

        let decoded = doc.decode_stream_with_encryption(&metadata, metadata_ref).unwrap();

        assert_eq!(decoded, PLAIN_METADATA);
    }

    fn decode_plain_metadata(pdf: Vec<u8>) -> Result<Vec<u8>> {
        let doc = PdfDocument::from_bytes(pdf).unwrap();
        let metadata_ref = ObjectRef::new(6, 0);
        let metadata = doc.load_object(metadata_ref).unwrap();
        doc.decode_stream_with_encryption(&metadata, metadata_ref)
    }

    #[test]
    fn named_non_identity_crypt_filter_is_rejected_instead_of_decrypted_with_stmf() {
        let pdf = encrypted_pdf_custom(
            Some("StdCF"),
            Some("/Type /Metadata /Subtype /XML /Filter [/Crypt] /DecodeParms [<< /Name /Other >>]"),
            None,
        );

        assert!(matches!(decode_plain_metadata(pdf), Err(Error::UnsupportedFilter(_))));
    }

    #[test]
    fn indirect_crypt_decode_parms_are_resolved_before_the_identity_check() {
        let named = encrypted_pdf_custom(
            Some("StdCF"),
            Some("/Type /Metadata /Subtype /XML /Filter [/Crypt] /DecodeParms [8 0 R]"),
            Some("<< /Name /Other >>"),
        );
        assert!(matches!(decode_plain_metadata(named), Err(Error::UnsupportedFilter(_))));

        let identity = encrypted_pdf_custom(
            Some("StdCF"),
            Some("/Type /Metadata /Subtype /XML /Filter [/Crypt] /DecodeParms [8 0 R]"),
            Some("<< /Name /Identity >>"),
        );
        assert_eq!(decode_plain_metadata(identity).unwrap(), PLAIN_METADATA);
    }

    #[test]
    fn null_crypt_name_is_treated_as_absent() {
        let pdf = encrypted_pdf_custom(
            Some("StdCF"),
            Some("/Type /Metadata /Subtype /XML /Filter [/Crypt] /DecodeParms [<< /Name null >>]"),
            None,
        );

        assert_eq!(decode_plain_metadata(pdf).unwrap(), PLAIN_METADATA);
    }

    #[test]
    fn absent_stmf_defaults_to_identity_while_strf_still_decrypts_strings() {
        let doc = PdfDocument::from_bytes(encrypted_pdf_custom(None, Some(IDENTITY_CRYPT_XMP_DICT), None)).unwrap();

        assert!(doc.extract_text(0).unwrap().contains("Plain body"));
        let info = doc.load_object(ObjectRef::new(5, 0)).unwrap();
        assert_eq!(
            info.as_dict().and_then(|d| d.get("Title")),
            Some(&Object::String(SECRET_TITLE.to_vec()))
        );
    }

    #[test]
    fn explicit_identity_and_none_methods_select_no_encryption() {
        let obj = make_encrypt_dict_obj("Standard", 4, 4, &[0u8; 32], &[0u8; 32], -1);
        let mut ed = EncryptDict::from_object(&obj).unwrap();

        ed.stream_crypt_method = Some("Identity".to_string());
        ed.string_crypt_method = Some("None".to_string());

        assert_eq!(ed.algorithm().unwrap(), Algorithm::None);
        assert_eq!(ed.string_algorithm().unwrap(), Algorithm::None);
    }

    #[test]
    fn r2_user_key_mismatch_in_the_second_half_is_rejected() {
        let key = algorithms::compute_encryption_key(b"", &[1u8; 32], PERMISSIONS, FILE_ID, 2, 5, true).unwrap();
        let mut user = algorithms::compute_user_password_hash(&key, FILE_ID, 2).unwrap();
        assert!(
            algorithms::authenticate_user_password(b"", &user, &[1u8; 32], PERMISSIONS, FILE_ID, 2, 5, true, None)
                .is_some()
        );

        user[24] ^= 0xFF;

        assert!(
            algorithms::authenticate_user_password(b"", &user, &[1u8; 32], PERMISSIONS, FILE_ID, 2, 5, true, None)
                .is_none()
        );
    }

    #[test]
    fn perms_block_verification_detects_altered_permissions() {
        let file_key = [0x42u8; 32];
        let mut block = Vec::new();
        block.extend_from_slice(&PERMISSIONS.to_le_bytes());
        block.extend_from_slice(&[0xFF; 4]);
        block.push(b'T');
        block.extend_from_slice(b"adb");
        block.extend_from_slice(&[9u8; 4]);
        let perms = aes::aes256_encrypt_no_padding(&file_key, &[0u8; 16], &block).unwrap();

        assert!(algorithms::verify_perms(&file_key, &perms, PERMISSIONS, true));
        assert!(!algorithms::verify_perms(&file_key, &perms, -1, true));
        assert!(!algorithms::verify_perms(&file_key, &perms, PERMISSIONS, false));

        let mut padded = perms.clone();
        padded.push(0);
        assert!(!algorithms::verify_perms(&file_key, &padded, PERMISSIONS, true));
    }

    #[test]
    fn handler_debug_output_omits_key_material() {
        let obj = make_encrypt_dict_obj("Standard", 4, 4, &[0xAB; 32], &[0xCD; 32], -1);
        let mut handler = handler::EncryptionHandler::new(&obj, FILE_ID.to_vec()).unwrap();
        let _ = handler.authenticate(b"");

        let rendered = format!("{handler:?}");

        assert!(!rendered.contains("171") && !rendered.contains("205"), "{rendered}");
        assert!(rendered.contains("EncryptionHandler"));
    }

    #[test]
    fn encrypted_xmp_stream_is_decrypted_before_parsing() {
        let doc = PdfDocument::from_bytes(encrypted_pdf_with_metadata("StdCF", true)).unwrap();

        let xmp = crate::extractors::xmp::XmpExtractor::extract(&doc).unwrap().unwrap();

        assert_eq!(xmp.pdf_producer.as_deref(), Some("Hidden Producer"));
    }
}
