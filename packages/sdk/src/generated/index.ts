import { PublicKey } from '@safecoin/web3.js';
export * from './accounts';
export * from './errors';
export * from './instructions';
export * from './types';

/**
 * Program address
 *
 * @category constants
 * @category generated
 */
export const PROGRAM_ADDRESS = 'autNTWWsmgHkTc9xGwaED2K7UMXB1YurFEuwiCKXpS9';

/**
 * Program public key
 *
 * @category constants
 * @category generated
 */
export const PROGRAM_ID = new PublicKey(PROGRAM_ADDRESS);
