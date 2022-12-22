#![cfg(feature = "test-bpf")]

pub mod utils;

use mpl_token_auth_rules::{
    error::RuleSetError,
    payload::{Payload, PayloadKey, PayloadType},
    state::{CompareOp, Rule, RuleSet},
};
use num_traits::cast::FromPrimitive;
use rmp_serde::Serializer;
use serde::Serialize;
use solana_program::instruction::{AccountMeta, InstructionError};
use solana_program_test::{tokio, BanksClientError};
use solana_sdk::{
    signature::Signer,
    signer::keypair::Keypair,
    transaction::{Transaction, TransactionError},
};
use utils::{program_test, Operation};

#[tokio::test]
async fn test_payer_not_signer_fails() {
    let mut context = program_test().start_with_context().await;

    // Find RuleSet PDA.
    let (rule_set_addr, _rule_set_bump) = mpl_token_auth_rules::pda::find_rule_set_address(
        context.payer.pubkey(),
        "test rule_set".to_string(),
    );

    // Create a `create` instruction.
    let create_ix = mpl_token_auth_rules::instruction::create(
        context.payer.pubkey(),
        rule_set_addr,
        vec![],
        vec![],
    );

    // Add it to a non-signed transaction.
    let create_tx = Transaction::new_with_payer(&[create_ix], Some(&context.payer.pubkey()));

    // Process the transaction.
    let err = context
        .banks_client
        .process_transaction(create_tx)
        .await
        .expect_err("creation should fail");

    // Deconstruct the error code and make sure it is what we expect.
    match err {
        BanksClientError::TransactionError(TransactionError::SignatureFailure) => (),
        _ => panic!("Unexpected error {:?}", err),
    }

    // Create a Keypair to simulate a token mint address.
    let mint = Keypair::new().pubkey();

    // Find RuleSet state PDA.
    let (rule_set_state_addr, _rule_set_bump) =
        mpl_token_auth_rules::pda::find_rule_set_state_address(
            context.payer.pubkey(),
            "test rule_set".to_string(),
            mint,
        );

    // Create a `validate` instruction.
    let validate_ix = mpl_token_auth_rules::instruction::validate(
        context.payer.pubkey(),
        context.payer.pubkey(),
        rule_set_addr,
        rule_set_state_addr,
        mint,
        Operation::Transfer.to_string(),
        Payload::default(),
        true,
        vec![],
    );

    // Add it to a non-signed transaction.
    let validate_tx = Transaction::new_with_payer(&[validate_ix], Some(&context.payer.pubkey()));

    // Process the transaction.
    let err = context
        .banks_client
        .process_transaction(validate_tx)
        .await
        .expect_err("validation should fail");

    // Deconstruct the error code and make sure it is what we expect.
    match err {
        BanksClientError::TransactionError(TransactionError::SignatureFailure) => (),
        _ => panic!("Unexpected error {:?}", err),
    }
}

#[tokio::test]
async fn test_additional_signer_and_amount() {
    let mut context = program_test().start_with_context().await;

    // Find RuleSet PDA.
    let (rule_set_addr, _rule_set_bump) = mpl_token_auth_rules::pda::find_rule_set_address(
        context.payer.pubkey(),
        "test rule_set".to_string(),
    );

    // Second signer.
    let second_signer = Keypair::new();

    // Create some rules.
    let adtl_signer = Rule::AdditionalSigner {
        account: context.payer.pubkey(),
    };
    let adtl_signer2 = Rule::AdditionalSigner {
        account: second_signer.pubkey(),
    };
    let amount_check = Rule::Amount {
        amount: 1,
        operator: CompareOp::Eq,
    };
    let not_amount_check = Rule::Not {
        rule: Box::new(amount_check),
    };

    let first_rule = Rule::All {
        rules: vec![adtl_signer, adtl_signer2],
    };

    let overall_rule = Rule::All {
        rules: vec![first_rule, not_amount_check],
    };

    // Create a RuleSet.
    let mut rule_set = RuleSet::new("test rule_set".to_string(), context.payer.pubkey());
    rule_set
        .add(Operation::Transfer.to_string(), overall_rule)
        .unwrap();

    println!("{:#?}", rule_set);

    // Serialize the RuleSet using RMP serde.
    let mut serialized_data = Vec::new();
    rule_set
        .serialize(&mut Serializer::new(&mut serialized_data))
        .unwrap();

    // Create a `create` instruction.
    let create_ix = mpl_token_auth_rules::instruction::create(
        context.payer.pubkey(),
        rule_set_addr,
        serialized_data,
        vec![],
    );

    // Add it to a transaction.
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    // Process the transaction.
    context
        .banks_client
        .process_transaction(create_tx)
        .await
        .expect("creation should succeed");

    // Create a Keypair to simulate a token mint address.
    let mint = Keypair::new().pubkey();

    // Find RuleSet state PDA.
    let (rule_set_state_addr, _rule_set_bump) =
        mpl_token_auth_rules::pda::find_rule_set_state_address(
            context.payer.pubkey(),
            "test rule_set".to_string(),
            mint,
        );

    // Store the payload of data to validate against the rule definition.
    let payload = Payload::from([(PayloadKey::Amount, PayloadType::Number(2))]);

    // Create a `validate` instruction WITHOUT the second signer.
    let validate_ix = mpl_token_auth_rules::instruction::validate(
        context.payer.pubkey(),
        context.payer.pubkey(),
        rule_set_addr,
        rule_set_state_addr,
        mint,
        Operation::Transfer.to_string(),
        payload.clone(),
        true,
        vec![AccountMeta::new_readonly(context.payer.pubkey(), true)],
    );

    // Add it to a transaction.
    let validate_tx = Transaction::new_signed_with_payer(
        &[validate_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    // Process the transaction.
    let err = context
        .banks_client
        .process_transaction(validate_tx)
        .await
        .expect_err("validation should fail");

    // Deconstruct the error code and make sure it is what we expect.
    match err {
        BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(val),
        )) => {
            let rule_set_error = RuleSetError::from_u32(val).unwrap();
            assert_eq!(rule_set_error, RuleSetError::MissingAccount);
        }
        _ => panic!("Unexpected error {:?}", err),
    }

    // Create a `validate` instruction WITH the second signer.
    let validate_ix = mpl_token_auth_rules::instruction::validate(
        context.payer.pubkey(),
        context.payer.pubkey(),
        rule_set_addr,
        rule_set_state_addr,
        mint,
        Operation::Transfer.to_string(),
        payload,
        true,
        vec![
            AccountMeta::new_readonly(context.payer.pubkey(), true),
            AccountMeta::new_readonly(second_signer.pubkey(), true),
        ],
    );

    // Add it to a transaction.
    let validate_tx = Transaction::new_signed_with_payer(
        &[validate_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &second_signer],
        context.last_blockhash,
    );

    // Process the transaction, this time it should succeed.
    context
        .banks_client
        .process_transaction(validate_tx)
        .await
        .expect("validation should succeed");

    // Store a payload of data with the WRONG amount.
    let payload = Payload::from([(PayloadKey::Amount, PayloadType::Number(1))]);

    // Create a `validate` instruction WITH the second signer.
    let validate_ix = mpl_token_auth_rules::instruction::validate(
        context.payer.pubkey(),
        context.payer.pubkey(),
        rule_set_addr,
        rule_set_state_addr,
        mint,
        Operation::Transfer.to_string(),
        payload,
        true,
        vec![
            AccountMeta::new_readonly(context.payer.pubkey(), true),
            AccountMeta::new_readonly(second_signer.pubkey(), true),
        ],
    );

    // Add it to a transaction.
    let validate_tx = Transaction::new_signed_with_payer(
        &[validate_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer, &second_signer],
        context.last_blockhash,
    );

    // Process the transaction.
    let err = context
        .banks_client
        .process_transaction(validate_tx)
        .await
        .expect_err("validation should fail");

    // Deconstruct the error code and make sure it is what we expect.
    match err {
        BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(val),
        )) => {
            let rule_set_error = RuleSetError::from_u32(val).unwrap();
            assert_eq!(rule_set_error, RuleSetError::AmountCheckFailed);
        }
        _ => panic!("Unexpected error {:?}", err),
    }
}

#[tokio::test]
async fn test_pass() {
    let mut context = program_test().start_with_context().await;

    // --------------------------------
    // Create RuleSet
    // --------------------------------
    // Find RuleSet PDA.
    let (rule_set_addr, _rule_set_bump) = mpl_token_auth_rules::pda::find_rule_set_address(
        context.payer.pubkey(),
        "test rule_set".to_string(),
    );

    // Create a Pass Rule.
    let pass_rule = Rule::Pass;

    // Create a RuleSet.
    let mut rule_set = RuleSet::new("test rule_set".to_string(), context.payer.pubkey());
    rule_set
        .add(Operation::Transfer.to_string(), pass_rule)
        .unwrap();

    println!("{:#?}", rule_set);

    // Serialize the RuleSet using RMP serde.
    let mut serialized_data = Vec::new();
    rule_set
        .serialize(&mut Serializer::new(&mut serialized_data))
        .unwrap();

    // Create a `create` instruction.
    let create_ix = mpl_token_auth_rules::instruction::create(
        context.payer.pubkey(),
        rule_set_addr,
        serialized_data,
        vec![],
    );

    // Add it to a transaction.
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    // Process the transaction.
    context
        .banks_client
        .process_transaction(create_tx)
        .await
        .expect("creation should succeed");

    // --------------------------------
    // Validate Pass Rule
    // --------------------------------
    // Warp some slots before validating.
    context.warp_to_slot(2).unwrap();

    // Create a Keypair to simulate a token mint address.
    let mint = Keypair::new().pubkey();

    // Find RuleSet state PDA.
    let (rule_set_state_addr, _rule_set_bump) =
        mpl_token_auth_rules::pda::find_rule_set_state_address(
            context.payer.pubkey(),
            "test rule_set".to_string(),
            mint,
        );

    // Create a `validate` instruction.
    let validate_ix = mpl_token_auth_rules::instruction::validate(
        context.payer.pubkey(),
        context.payer.pubkey(),
        rule_set_addr,
        rule_set_state_addr,
        mint,
        Operation::Transfer.to_string(),
        Payload::default(),
        true,
        vec![],
    );

    // Add it to a transaction.
    let validate_tx = Transaction::new_signed_with_payer(
        &[validate_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    // Process the transaction.
    context
        .banks_client
        .process_transaction(validate_tx)
        .await
        .expect("validation should succeed");
}
