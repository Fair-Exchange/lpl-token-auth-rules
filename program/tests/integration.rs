#![cfg(feature = "test-bpf")]

pub mod utils;

use borsh::BorshSerialize;
use mpl_token_auth_rules::{
    error::RuleSetError,
    instruction::{
        builders::{CreateOrUpdateBuilder, ValidateBuilder},
        CreateOrUpdateArgs, InstructionBuilder, ValidateArgs,
    },
    payload::{Payload, PayloadType},
    state::{
        CompareOp, Rule, RuleSetHeader, RuleSetRevisionMapV1, RuleSetV1, RULE_SET_LIB_VERSION,
        RULE_SET_REV_MAP_VERSION, RULE_SET_SERIALIZED_HEADER_LEN,
    },
};
use rmp_serde::Serializer;
use serde::Serialize;
use solana_program::instruction::AccountMeta;
use solana_program_test::tokio;
use solana_sdk::{signature::Signer, signer::keypair::Keypair, transaction::Transaction};
use utils::{cmp_slice, program_test, Operation, PayloadKey};

#[tokio::test]
#[should_panic]
async fn test_payer_not_signer_panics() {
    let mut context = program_test().start_with_context().await;

    // --------------------------------
    // Create RuleSet
    // --------------------------------
    // Create a Pass Rule.
    let pass_rule = Rule::Pass;

    // Create a RuleSet.
    let other_payer = Keypair::new();
    let mut rule_set = RuleSetV1::new("test rule_set".to_string(), other_payer.pubkey());
    rule_set
        .add(Operation::OwnerTransfer.to_string(), pass_rule)
        .unwrap();

    // Find RuleSet PDA.
    let (rule_set_addr, _rule_set_bump) = mpl_token_auth_rules::pda::find_rule_set_address(
        other_payer.pubkey(),
        "test rule_set".to_string(),
    );

    // Serialize the RuleSet using RMP serde.
    let mut serialized_rule_set = Vec::new();
    rule_set
        .serialize(&mut Serializer::new(&mut serialized_rule_set))
        .unwrap();

    // Create a `create` instruction with a payer that won't be a signer.
    let create_ix = CreateOrUpdateBuilder::new()
        .payer(other_payer.pubkey())
        .rule_set_pda(rule_set_addr)
        .build(CreateOrUpdateArgs::V1 {
            serialized_rule_set,
        })
        .unwrap()
        .instruction();

    // Add it to a transaction but don't add other payer as a signer.
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    // Process the transaction.  It will panic because of not enough signers.
    let _result = context.banks_client.process_transaction(create_tx).await;
}

#[tokio::test]
async fn test_composed_rule() {
    let mut context = program_test().start_with_context().await;

    // --------------------------------
    // Create RuleSet
    // --------------------------------
    // Create some rules.
    let adtl_signer = Rule::AdditionalSigner {
        account: context.payer.pubkey(),
    };

    // Second signer.
    let second_signer = Keypair::new();

    let adtl_signer2 = Rule::AdditionalSigner {
        account: second_signer.pubkey(),
    };
    let amount_check = Rule::Amount {
        amount: 1,
        operator: CompareOp::Eq,
        field: PayloadKey::Amount.to_string(),
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
    let mut rule_set = RuleSetV1::new("test rule_set".to_string(), context.payer.pubkey());
    rule_set
        .add(Operation::OwnerTransfer.to_string(), overall_rule)
        .unwrap();

    println!("{:#?}", rule_set);

    // Put the RuleSet on chain.
    let rule_set_addr =
        create_rule_set_on_chain!(&mut context, rule_set, "test rule_set".to_string()).await;

    // --------------------------------
    // Validate fail missing account
    // --------------------------------
    // Create a Keypair to simulate a token mint address.
    let mint = Keypair::new().pubkey();

    // Store a payload of data with an amount not allowed by the Amount Rule (Amount Rule NOT'd).
    let payload = Payload::from([(PayloadKey::Amount.to_string(), PayloadType::Number(2))]);

    // Create a `validate` instruction WITHOUT the second signer.
    let validate_ix = ValidateBuilder::new()
        .rule_set_pda(rule_set_addr)
        .mint(mint)
        .additional_rule_accounts(vec![AccountMeta::new_readonly(
            context.payer.pubkey(),
            true,
        )])
        .build(ValidateArgs::V1 {
            operation: Operation::OwnerTransfer.to_string(),
            payload: payload.clone(),
            update_rule_state: false,
            rule_set_revision: None,
        })
        .unwrap()
        .instruction();

    // Fail to validate Transfer operation.
    let err = process_failing_validate_ix!(&mut context, validate_ix, vec![], None).await;

    // Check that error is what we expect.
    assert_custom_error!(err, RuleSetError::MissingAccount);

    // --------------------------------
    // Validate pass
    // --------------------------------
    // Create a `validate` instruction WITH the second signer.
    let validate_ix = ValidateBuilder::new()
        .rule_set_pda(rule_set_addr)
        .mint(mint)
        .additional_rule_accounts(vec![
            AccountMeta::new_readonly(context.payer.pubkey(), true),
            AccountMeta::new_readonly(second_signer.pubkey(), true),
        ])
        .build(ValidateArgs::V1 {
            operation: Operation::OwnerTransfer.to_string(),
            payload,
            update_rule_state: false,
            rule_set_revision: None,
        })
        .unwrap()
        .instruction();

    // Validate Transfer operation.
    process_passing_validate_ix!(&mut context, validate_ix, vec![&second_signer], None).await;

    // --------------------------------
    // Validate fail wrong amount
    // --------------------------------
    // Store a payload of data with an amount allowed by the Amount Rule (Amount Rule NOT'd).
    let payload = Payload::from([(PayloadKey::Amount.to_string(), PayloadType::Number(1))]);

    // Create a `validate` instruction WITH the second signer.  Will fail as Amount Rule is NOT'd.
    let validate_ix = ValidateBuilder::new()
        .rule_set_pda(rule_set_addr)
        .mint(mint)
        .additional_rule_accounts(vec![
            AccountMeta::new_readonly(context.payer.pubkey(), true),
            AccountMeta::new_readonly(second_signer.pubkey(), true),
        ])
        .build(ValidateArgs::V1 {
            operation: Operation::OwnerTransfer.to_string(),
            payload,
            update_rule_state: false,
            rule_set_revision: None,
        })
        .unwrap()
        .instruction();

    // Fail to validate Transfer operation.
    let err =
        process_failing_validate_ix!(&mut context, validate_ix, vec![&second_signer], None).await;

    // Check that error is what we expect.
    assert_custom_error!(err, RuleSetError::AmountCheckFailed);
}

#[tokio::test]
async fn test_update_ruleset() {
    let mut context = program_test().start_with_context().await;

    // --------------------------------
    // Create RuleSet
    // --------------------------------
    // Create a Pass Rule.
    let pass_rule = Rule::Pass;

    // Create a RuleSet.
    let mut first_rule_set = RuleSetV1::new("test rule_set".to_string(), context.payer.pubkey());
    first_rule_set
        .add(Operation::OwnerTransfer.to_string(), pass_rule)
        .unwrap();

    // Put the RuleSet on chain.
    let _rule_set_addr = create_rule_set_on_chain!(
        &mut context,
        first_rule_set.clone(),
        "test rule_set".to_string()
    )
    .await;

    // --------------------------------
    // Update RuleSet
    // --------------------------------
    // Create some other rules.
    let adtl_signer = Rule::AdditionalSigner {
        account: context.payer.pubkey(),
    };

    let amount_check = Rule::Amount {
        amount: 1,
        operator: CompareOp::Eq,
        field: PayloadKey::Amount.to_string(),
    };

    let overall_rule = Rule::All {
        rules: vec![adtl_signer, amount_check],
    };

    // Create a new RuleSet.
    let mut second_rule_set = RuleSetV1::new("test rule_set".to_string(), context.payer.pubkey());
    second_rule_set
        .add(Operation::OwnerTransfer.to_string(), overall_rule)
        .unwrap();

    // Put the updated RuleSet on chain.
    let rule_set_addr = create_rule_set_on_chain!(
        &mut context,
        second_rule_set.clone(),
        "test rule_set".to_string()
    )
    .await;

    // --------------------------------
    // Validate the on chain data
    // --------------------------------
    // Get the `RuleSet` PDA data.
    let data = context
        .banks_client
        .get_account(rule_set_addr)
        .await
        .unwrap()
        .unwrap()
        .data;

    // Check the first `RuleSet` lib version.
    let first_rule_set_version_loc = RULE_SET_SERIALIZED_HEADER_LEN;
    assert_eq!(
        data[first_rule_set_version_loc], RULE_SET_LIB_VERSION,
        "The buffer doesn't match the first rule set's lib version"
    );

    // Serialize the first `RuleSet` using RMP serde.
    let mut serialized_first_rule_set = Vec::new();
    first_rule_set
        .serialize(&mut Serializer::new(&mut serialized_first_rule_set))
        .unwrap();

    // Check the first `RuleSet` serialized data.
    let first_rule_set_start = first_rule_set_version_loc + 1;
    let first_rule_set_end = first_rule_set_start + serialized_first_rule_set.len();
    assert!(
        cmp_slice(
            &data[first_rule_set_start..first_rule_set_end],
            &serialized_first_rule_set
        ),
        "The buffer doesn't match the serialized first rule set.",
    );

    // Check the second `RuleSet` lib version.
    let second_rule_set_version_loc = first_rule_set_end;
    assert_eq!(
        data[second_rule_set_version_loc], RULE_SET_LIB_VERSION,
        "The buffer doesn't match the second rule set's lib version"
    );

    // Serialize the second `RuleSet` using RMP serde.
    let mut serialized_second_rule_set = Vec::new();
    second_rule_set
        .serialize(&mut Serializer::new(&mut serialized_second_rule_set))
        .unwrap();

    // Check the second `RuleSet` serialized data.
    let second_rule_set_start = second_rule_set_version_loc + 1;
    let second_rule_set_end = second_rule_set_start + serialized_second_rule_set.len();
    assert!(
        cmp_slice(
            &data[second_rule_set_start..second_rule_set_end],
            &serialized_second_rule_set
        ),
        "The buffer doesn't match the serialized second rule set.",
    );

    // Check the revision map version.
    let rev_map_version_loc = second_rule_set_end;
    assert_eq!(
        data[rev_map_version_loc], RULE_SET_REV_MAP_VERSION,
        "The buffer doesn't match the revision map version"
    );

    // Create revision map using the known locations of the two `RuleSet`s in this test.
    let mut revision_map = RuleSetRevisionMapV1::default();
    revision_map
        .rule_set_revisions
        .push(first_rule_set_version_loc);
    revision_map
        .rule_set_revisions
        .push(second_rule_set_version_loc);

    // Borsh serialize the revision map.
    let mut serialized_rev_map = Vec::new();
    revision_map.serialize(&mut serialized_rev_map).unwrap();

    // Check the revision map.
    let rev_map_start = rev_map_version_loc + 1;
    assert!(
        cmp_slice(&data[rev_map_start..], &serialized_rev_map),
        "The buffer doesn't match the serialized revision map.",
    );

    // Create header using the known location of the revision map version location.
    let header = RuleSetHeader::new(rev_map_version_loc);

    // Borsh serialize the header.
    let mut serialized_header = Vec::new();
    header.serialize(&mut serialized_header).unwrap();

    // Check the header.
    assert!(
        cmp_slice(&data[..RULE_SET_SERIALIZED_HEADER_LEN], &serialized_header),
        "The buffer doesn't match the serialized header.",
    );
}
