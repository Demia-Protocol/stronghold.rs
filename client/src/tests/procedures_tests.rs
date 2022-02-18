// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(non_snake_case)]

use std::convert::TryInto;

use crypto::signatures::ed25519;
use stronghold_utils::random::bytestring;

use super::fresh;
use crate::{
    procedures::{
        BIP39Generate, ChainCode, Ed25519Sign, KeyType, MnemonicLanguage, PublicKey, Slip10Derive, Slip10Generate,
        Slip10ParentType,
    },
    state::secure::SecureClient,
    Location, Stronghold,
};

async fn setup_stronghold() -> (Vec<u8>, Stronghold) {
    let cp = fresh::bytestring(u8::MAX.into());

    let s = Stronghold::init_stronghold_system(cp.clone(), vec![]).await.unwrap();
    (cp, s)
}

#[actix::test]
async fn usecase_ed25519() {
    let (_cp, sh) = setup_stronghold().await;

    let vault_path = bytestring(1024);
    let seed = Location::generic(vault_path.clone(), bytestring(1024));
    let seed_hint = fresh::record_hint();

    if fresh::coinflip() {
        let size_bytes = if fresh::coinflip() {
            Some(fresh::usize(1024))
        } else {
            None
        };
        let slip10_generate = Slip10Generate {
            size_bytes,
            output: seed.clone(),
            hint: seed_hint,
        };

        match sh.runtime_exec(slip10_generate).await.unwrap() {
            Ok(_) => (),
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    } else {
        let bip32_gen = BIP39Generate {
            passphrase: fresh::passphrase(),
            output: seed.clone(),
            hint: seed_hint,
            language: MnemonicLanguage::English,
        };
        match sh.runtime_exec(bip32_gen).await.unwrap() {
            Ok(_) => (),
            Err(err) => panic!("unexpected error: {:?}", err),
        }
    }

    let (_path, chain) = fresh::hd_path();
    let key = Location::generic(vault_path.clone(), bytestring(1024));
    let key_hint = fresh::record_hint();

    let slip10_derive = Slip10Derive {
        chain,
        input: seed.clone(),
        parent_ty: Slip10ParentType::Seed,
        output: key.clone(),
        hint: key_hint,
    };
    match sh.runtime_exec(slip10_derive).await.unwrap() {
        Ok(_) => (),
        Err(err) => panic!("unexpected error: {:?}", err),
    };

    let ed25519_pk = PublicKey {
        private_key: key.clone(),
        ty: KeyType::Ed25519,
    };
    let pk: [u8; ed25519::PUBLIC_KEY_LENGTH] = match sh.runtime_exec(ed25519_pk).await.unwrap() {
        Ok(data) => data.try_into().unwrap(),
        Err(e) => panic!("unexpected error: {:?}", e),
    };

    let msg = fresh::bytestring(4096);

    let ed25519_sign = Ed25519Sign {
        private_key: key.clone(),
        msg: msg.clone(),
    };
    let sig: [u8; ed25519::SIGNATURE_LENGTH] = match sh.runtime_exec(ed25519_sign).await.unwrap() {
        Ok(data) => data,
        Err(e) => panic!("unexpected error: {:?}", e),
    };

    let pk = ed25519::PublicKey::try_from_bytes(pk).unwrap();
    let sig = ed25519::Signature::from_bytes(sig);
    assert!(pk.verify(&sig, &msg));

    let list = sh.list_hints_and_ids(vault_path).await.unwrap();
    assert_eq!(list.len(), 2);
    let (_, hint) = list
        .iter()
        .find(|(id, _)| *id == SecureClient::resolve_location(seed.clone()).1)
        .unwrap();
    assert_eq!(*hint, seed_hint);
    let (_, hint) = list
        .iter()
        .find(|(id, _)| *id == SecureClient::resolve_location(key.clone()).1)
        .unwrap();
    assert_eq!(*hint, key_hint);
}

#[actix::test]
async fn usecase_Slip10Derive_intermediate_keys() {
    let (_cp, sh) = setup_stronghold().await;

    let seed = fresh::location();

    let slip10_generate = Slip10Generate {
        output: seed.clone(),
        hint: fresh::record_hint(),
        size_bytes: None,
    };
    match sh.runtime_exec(slip10_generate).await.unwrap() {
        Ok(_) => (),
        Err(e) => panic!("unexpected error: {:?}", e),
    };

    let (_path, chain0) = fresh::hd_path();
    let (_path, chain1) = fresh::hd_path();

    let cc0: ChainCode = {
        let slip10_derive = Slip10Derive {
            input: seed.clone(),
            chain: chain0.join(&chain1),
            output: fresh::location(),
            hint: fresh::record_hint(),
            parent_ty: Slip10ParentType::Seed,
        };

        match sh.runtime_exec(slip10_derive).await.unwrap() {
            Ok(data) => data,
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    };

    let cc1: ChainCode = {
        let intermediate = fresh::location();

        let slip10_derive_intermediate = Slip10Derive {
            input: seed.clone(),
            chain: chain0,
            output: intermediate.clone(),
            hint: fresh::record_hint(),
            parent_ty: Slip10ParentType::Seed,
        };

        match sh.runtime_exec(slip10_derive_intermediate).await.unwrap() {
            Ok(_) => (),
            Err(e) => panic!("unexpected error: {:?}", e),
        };

        let slip10_derive_child = Slip10Derive {
            input: intermediate,
            chain: chain1,
            output: fresh::location(),
            hint: fresh::record_hint(),
            parent_ty: Slip10ParentType::Key,
        };

        match sh.runtime_exec(slip10_derive_child).await.unwrap() {
            Ok(data) => data,
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    };

    assert_eq!(cc0, cc1);
}

// #[actix::test]
// async fn usecase_ed25519_as_complex() {
//     let (_cp, sh) = setup_stronghold().await;

//     let msg = fresh::bytestring(4096);

//     let pk_result = OutputKey::random();
//     let sign_result = OutputKey::random();

//     let generate = Slip10Generate::default();
//     let derive = Slip10Derive::new_from_seed(generate.target(), fresh::hd_path().1);
//     let get_pk = PublicKey::new(KeyType::Ed25519, derive.target());
//     let sign = Ed25519Sign::new(msg.clone(), derive.target());

//     let combined_proc = generate.then(derive).then(get_pk).then(sign);
//     let mut output = match sh.runtime_exec(combined_proc).await.unwrap() {
//         Ok(o) => o,
//         Err(e) => panic!("Unexpected error: {}", e),
//     };

//     let pub_key_vec: [u8; ed25519::PUBLIC_KEY_LENGTH] = output.take(&pk_result).unwrap();
//     let pk = ed25519::PublicKey::try_from_bytes(pub_key_vec).unwrap();
//     let sig_vec: [u8; ed25519::SIGNATURE_LENGTH] = output.take(&sign_result).unwrap();
//     let sig = ed25519::Signature::from_bytes(sig_vec);
//     assert!(pk.verify(&sig, &msg));
// }

// #[actix::test]
// async fn usecase_collection_of_data() {
//     let (_cp, sh) = setup_stronghold().await;

//     let key: Vec<u8> = {
//         let size_bytes = fresh::coinflip().then(|| fresh::usize(1024)).unwrap_or(64);
//         let mut seed = vec![0u8; size_bytes];
//         fill(&mut seed).unwrap();
//         let dk = slip10::Seed::from_bytes(&seed)
//             .derive(slip10::Curve::Ed25519, &fresh::hd_path().1)
//             .unwrap();
//         dk.into()
//     };

//     // write seed to vault
//     let key_location = fresh::location();
//     sh.write_to_vault(key_location.clone(), key.clone(), fresh::record_hint(), Vec::new())
//         .await
//         .unwrap()
//         .unwrap_or_else(|e| panic!("Unexpected error: {}", e));

//     // test sign and hash

//     let messages = vec![Vec::from("msg1"), Vec::from("msg2"), Vec::from("msg3")];

//     let expected = messages
//         .clone()
//         .into_iter()
//         .map(|msg| {
//             // Sign message
//             let mut raw = key.clone();
//             raw.truncate(32);
//             let mut bs = [0; 32];
//             bs.copy_from_slice(&raw);
//             let sk = ed25519::SecretKey::from_bytes(bs);
//             let sig = sk.sign(&msg).to_bytes();

//             // SHA-256 hash the signed message
//             let mut digest = [0; SHA256_LEN];
//             SHA256(&sig, &mut digest);
//             digest
//         })
//         .filter(|bytes| bytes.iter().any(|b| b <= &10u8))
//         .fold(Vec::new(), |mut acc, curr| {
//             acc.extend_from_slice(&curr);
//             acc
//         });

//     // test procedure
//     let proc = messages
//         .into_iter()
//         .enumerate()
//         .map(|(i, msg)| {
//             let sign = Ed25519Sign::new(msg, key_location.clone());
//             let digest = Hash::new(HashType::Sha2(Sha2Hash::Sha256), sign.output_key())
//                 ;
//             sign.then(digest)
//         })
//         .reduce(|acc, curr| acc.then(curr))
//         .unwrap();
//     let mut output = match sh.runtime_exec(proc).await.unwrap() {
//         Ok(o) => o.into_iter().collect::<Vec<(OutputKey, ProcedureIo)>>(),
//         Err(e) => panic!("Unexpected error: {}", e),
//     };
//     output.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
//     let res = output
//         .into_iter()
//         .map(|(_, v)| v)
//         .filter(|bytes| bytes.iter().any(|b| b <= &10u8))
//         .fold(Vec::new(), |mut acc, curr| {
//             acc.extend_from_slice(&curr);
//             acc
//         });
//     assert_eq!(res, expected);
// }

// async fn test_aead(sh: &mut Stronghold, key_location: Location, key: &[u8], alg: AeadAlg) {
//     let test_plaintext = random::bytestring(4096);
//     let test_associated_data = random::bytestring(4096);
//     let nonce_len = match alg {
//         AeadAlg::Aes256Gcm => Aes256Gcm::NONCE_LENGTH,
//         AeadAlg::XChaCha20Poly1305 => XChaCha20Poly1305::NONCE_LENGTH,
//     };
//     let mut test_nonce = Vec::with_capacity(nonce_len);
//     for _ in 0..test_nonce.capacity() {
//         test_nonce.push(random::random())
//     }

//     // test encryption
//     let ctx_key = OutputKey::new("ctx");
//     let tag_key = OutputKey::new("tag");
//     let aead = AeadEncrypt::new(
//         alg,
//         key_location.clone(),
//         test_plaintext.clone(),
//         test_associated_data.clone(),
//         test_nonce.to_vec(),
//     )
//     .store_ciphertext(ctx_key.clone())
//     .store_tag(tag_key.clone());

//     let mut output = sh
//         .runtime_exec(aead)
//         .await
//         .unwrap()
//         .unwrap_or_else(|e| panic!("Unexpected error: {}", e));
//     let out_ciphertext: Vec<u8> = output.take(&ctx_key).unwrap();
//     let out_tag: Vec<u8> = output.take(&tag_key).unwrap();

//     let mut expected_ctx = vec![0; test_plaintext.len()];
//     let tag_len = match alg {
//         AeadAlg::Aes256Gcm => Aes256Gcm::TAG_LENGTH,
//         AeadAlg::XChaCha20Poly1305 => XChaCha20Poly1305::TAG_LENGTH,
//     };
//     let mut expected_tag = vec![0; tag_len];

//     let f = match alg {
//         AeadAlg::Aes256Gcm => Aes256Gcm::try_encrypt,
//         AeadAlg::XChaCha20Poly1305 => XChaCha20Poly1305::try_encrypt,
//     };
//     f(
//         key,
//         &test_nonce,
//         &test_associated_data,
//         &test_plaintext,
//         &mut expected_ctx,
//         &mut expected_tag,
//     )
//     .unwrap_or_else(|e| panic!("Unexpected error: {}", e));

//     assert_eq!(expected_ctx, out_ciphertext);
//     assert_eq!(expected_tag, out_tag);

//     // test decryption
//     let ptx_key = OutputKey::new("ptx");
//     let adad = AeadDecrypt::new(
//         alg,
//         key_location,
//         out_ciphertext.clone(),
//         test_associated_data.clone(),
//         out_tag.clone(),
//         test_nonce.to_vec(),
//     )
//     .store_plaintext(ptx_key.clone());

//     let mut output = sh
//         .runtime_exec(adad)
//         .await
//         .unwrap()
//         .unwrap_or_else(|e| panic!("Unexpected error: {}", e));
//     let out_plaintext: Vec<u8> = output.take(&ptx_key).unwrap();

//     let mut expected_ptx = vec![0; out_ciphertext.len()];

//     let f = match alg {
//         AeadAlg::Aes256Gcm => Aes256Gcm::try_decrypt,
//         AeadAlg::XChaCha20Poly1305 => XChaCha20Poly1305::try_decrypt,
//     };
//     f(
//         key,
//         &test_nonce,
//         &test_associated_data,
//         &mut expected_ptx,
//         &out_ciphertext,
//         &out_tag,
//     )
//     .unwrap_or_else(|e| panic!("Unexpected error: {}", e));

//     assert_eq!(expected_ptx, out_plaintext);
//     assert_eq!(out_plaintext, test_plaintext);
// }

// #[actix::test]
// async fn usecase_aead() {
//     let (_cp, mut sh) = setup_stronghold().await;

//     // Init key
//     let key_location = fresh::location();
//     let key = ed25519::SecretKey::generate().unwrap().to_bytes();
//     sh.write_to_vault(key_location.clone(), key.to_vec(), fresh::record_hint(), Vec::new())
//         .await
//         .unwrap()
//         .unwrap_or_else(|e| panic!("Unexpected error: {}", e));

//     test_aead(&mut sh, key_location.clone(), &key, AeadAlg::Aes256Gcm).await;
//     test_aead(&mut sh, key_location.clone(), &key, AeadAlg::XChaCha20Poly1305).await;
// }

// #[actix::test]
// async fn usecase_diffie_hellman() {
//     let (cp, sh) = setup_stronghold().await;

//     let sk1_location = fresh::location();
//     let sk1 = GenerateKey::new(KeyType::X25519).write_secret(sk1_location.clone(), fresh::record_hint());
//     let pk1 = PublicKey::new(KeyType::X25519, sk1.target());
//     let pub_key_1: [u8; 32] = sh
//         .runtime_exec(sk1.then(pk1))
//         .await
//         .unwrap()
//         .unwrap()
//         .single_output()
//         .unwrap();

//     let sk2_location = fresh::location();
//     let sk2 = GenerateKey::new(KeyType::X25519).write_secret(sk2_location.clone(), fresh::record_hint());
//     let pk2 = PublicKey::new(KeyType::X25519, sk2.target());
//     let pub_key_2: [u8; 32] = sh
//         .runtime_exec(sk2.then(pk2))
//         .await
//         .unwrap()
//         .unwrap()
//         .single_output()
//         .unwrap();

//     let mut salt = vec![];
//     salt.extend_from_slice(&pub_key_1);
//     salt.extend_from_slice(&pub_key_2);
//     let label = bytestring(1024);

//     let key_1_2 = fresh::location();
//     let dh_1_2 = X25519DiffieHellman::new(pub_key_2, sk1_location);
//     let derived_1_2 = Hkdf::new(Sha2Hash::Sha256, salt.clone(), label.clone(), dh_1_2.target())
//         .write_secret(key_1_2.clone(), fresh::record_hint());

//     let key_2_1 = fresh::location();
//     let dh_2_1 = X25519DiffieHellman::new(pub_key_1, sk2_location);
//     let derived_2_1 = Hkdf::new(Sha2Hash::Sha256, salt.clone(), label.clone(), dh_2_1.target())
//         .write_secret(key_2_1.clone(), fresh::record_hint());

//     sh.runtime_exec(dh_1_2.then(derived_1_2).then(dh_2_1).then(derived_2_1))
//         .await
//         .unwrap()
//         .unwrap_or_else(|e| panic!("Unexpected error: {}", e));

//     let hashed_shared_1_2 = sh.read_secret(cp.clone(), key_1_2).await.unwrap().unwrap();
//     let hashed_shared_2_1 = sh.read_secret(cp, key_2_1).await.unwrap().unwrap();

//     assert_eq!(hashed_shared_1_2, hashed_shared_2_1)
// }

// #[actix::test]
// async fn usecase_recover_bip39() {
//     let (_cp, sh) = setup_stronghold().await;

//     let passphrase = string(4096);
//     let (_path, chain) = fresh::hd_path();
//     let message = bytestring(4095);

//     let generate_bip39 = BIP39Generate::new(MnemonicLanguage::English, Some(passphrase.clone()));
//     let derive_from_original = Slip10Derive::new_from_seed(generate_bip39.target(), chain.clone());
//     let signed_with_original = OutputKey::new("original");
//     let sign_from_original =
//         Ed25519Sign::new(message.clone(), derive_from_original.target());

//     let recover_bip39 = BIP39Recover::new(generate_bip39.output_key(), Some(passphrase));
//     let derive_from_recovered = Slip10Derive::new_from_seed(recover_bip39.target(), chain.clone());
//     let signed_with_recovered = OutputKey::new("recovered");
//     let sign_from_recovered =
//         Ed25519Sign::new(message, derive_from_recovered.target());

//     let proc = generate_bip39
//         .then(derive_from_original)
//         .then(sign_from_original)
//         .then(recover_bip39)
//         .then(derive_from_recovered)
//         .then(sign_from_recovered);
//     let mut output = sh
//         .runtime_exec(proc)
//         .await
//         .unwrap()
//         .unwrap_or_else(|e| panic!("Unexpected error: {}", e));
//     let with_original: Vec<u8> = output.take(&signed_with_original).unwrap();
//     let with_recovered: Vec<u8> = output.take(&signed_with_recovered).unwrap();
//     assert_eq!(with_original, with_recovered);
// }

// #[actix::test]
// async fn usecase_move_record() {
//     let (_cp, sh) = setup_stronghold().await;
//     let test_msg = random::bytestring(4096);

//     let first_location = fresh::location();
//     let generate_key = GenerateKey::new(KeyType::Ed25519).write_secret(first_location.clone(), fresh::record_hint());
//     let pub_key = PublicKey::new(KeyType::Ed25519, first_location.clone());
//     let sign_message =
//         Ed25519Sign::new(test_msg.clone(), first_location.clone());
//     let mut output = sh
//         .runtime_exec(generate_key.then(pub_key).then(sign_message))
//         .await
//         .unwrap()
//         .unwrap_or_else(|e| panic!("Unexpected error: {}", e));
//     // signed message used for validation further in the test
//     let signed_with_original: Vec<u8> = output.take(&OutputKey::new("signed")).unwrap();

//     // pub-key used to derive the new location for the private key
//     let public_key = output.take(&OutputKey::new("pub-key")).unwrap();
//     let mut first: Vec<u8> = public_key;
//     let second: Vec<u8> = first.drain(first.len() % 2..).collect();

//     // Copy record to new location derived from the pub-key
//     let new_location = Location::generic(first, second);
//     let copy_record = CopyRecord::new(first_location.clone()).write_secret(new_location.clone(),
// fresh::record_hint());     sh.runtime_exec(copy_record)
//         .await
//         .unwrap()
//         .unwrap_or_else(|e| panic!("Unexpected error: {}", e));
//     // Remove record from old location
//     sh.delete_data(first_location, true)
//         .await
//         .unwrap()
//         .unwrap_or_else(|e| panic!("Unexpected error: {}", e));

//     // Validate by signing the message from the new location
//     let sign_message = Ed25519Sign::new(test_msg, new_location);
//     let signed_with_moved: Vec<u8> = sh
//         .runtime_exec(sign_message)
//         .await
//         .unwrap()
//         .unwrap_or_else(|e| panic!("Unexpected error: {}", e))
//         .try_into()
//         .unwrap();
//     assert_eq!(signed_with_original, signed_with_moved);
// }
