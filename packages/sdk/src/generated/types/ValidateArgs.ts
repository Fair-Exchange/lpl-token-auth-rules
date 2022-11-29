/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet';
import { Operation, operationBeet } from './Operation';
import { Payload, payloadBeet } from './Payload';
export type ValidateArgs = {
  name: string;
  operation: Operation;
  payloadMap: Map<number, Payload>;
};

/**
 * @category userTypes
 * @category generated
 */
export const validateArgsBeet = new beet.FixableBeetArgsStruct<ValidateArgs>(
  [
    ['name', beet.utf8String],
    ['operation', operationBeet],
    ['payloadMap', beet.map(beet.u8, payloadBeet)],
  ],
  'ValidateArgs',
);
