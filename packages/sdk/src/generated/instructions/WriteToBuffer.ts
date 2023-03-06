/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet';
import * as web3 from '@safecoin/web3.js';
import { WriteToBufferArgs, writeToBufferArgsBeet } from '../types/WriteToBufferArgs';

/**
 * @category Instructions
 * @category WriteToBuffer
 * @category generated
 */
export type WriteToBufferInstructionArgs = {
  writeToBufferArgs: WriteToBufferArgs;
};
/**
 * @category Instructions
 * @category WriteToBuffer
 * @category generated
 */
export const WriteToBufferStruct = new beet.FixableBeetArgsStruct<
  WriteToBufferInstructionArgs & {
    instructionDiscriminator: number;
  }
>(
  [
    ['instructionDiscriminator', beet.u8],
    ['writeToBufferArgs', writeToBufferArgsBeet],
  ],
  'WriteToBufferInstructionArgs',
);
/**
 * Accounts required by the _WriteToBuffer_ instruction
 *
 * @property [_writable_, **signer**] payer Payer and creator of the RuleSet
 * @property [_writable_] bufferPda The PDA account where the RuleSet buffer is stored
 * @category Instructions
 * @category WriteToBuffer
 * @category generated
 */
export type WriteToBufferInstructionAccounts = {
  payer: web3.PublicKey;
  bufferPda: web3.PublicKey;
  systemProgram?: web3.PublicKey;
};

export const writeToBufferInstructionDiscriminator = 2;

/**
 * Creates a _WriteToBuffer_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @param args to provide as instruction data to the program
 *
 * @category Instructions
 * @category WriteToBuffer
 * @category generated
 */
export function createWriteToBufferInstruction(
  accounts: WriteToBufferInstructionAccounts,
  args: WriteToBufferInstructionArgs,
  programId = new web3.PublicKey('autNTWWsmgHkTc9xGwaED2K7UMXB1YurFEuwiCKXpS9'),
) {
  const [data] = WriteToBufferStruct.serialize({
    instructionDiscriminator: writeToBufferInstructionDiscriminator,
    ...args,
  });
  const keys: web3.AccountMeta[] = [
    {
      pubkey: accounts.payer,
      isWritable: true,
      isSigner: true,
    },
    {
      pubkey: accounts.bufferPda,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: accounts.systemProgram ?? web3.SystemProgram.programId,
      isWritable: false,
      isSigner: false,
    },
  ];

  const ix = new web3.TransactionInstruction({
    programId,
    keys,
    data,
  });
  return ix;
}
