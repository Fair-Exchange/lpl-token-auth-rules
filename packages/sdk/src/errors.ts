import { initCusper } from '@safecoin/cusper';
// @ts-ignore
import { errorFromCode } from './generated';

export const cusper = initCusper(errorFromCode);
