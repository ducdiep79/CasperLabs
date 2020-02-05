import * as CL from "../../../../contract-as/assembly";
import {Error, ErrorCode} from "../../../../contract-as/assembly/error";
import { fromBytesU64, fromBytesArrayU8, GetLastError, Error as BytesreprError} from "../../../../contract-as/assembly/bytesrepr";
import {transferToAccount} from "../../../../contract-as/assembly";
import {U512} from "../../../../contract-as/assembly/bignum";

export function call(): void {
  let accountBytes = CL.getArg(0);
  if (accountBytes === null) {
    Error.fromErrorCode(ErrorCode.MissingArgument).revert();
    return;
  }

  let amountBytes = CL.getArg(1);
  if (amountBytes === null) {
    Error.fromErrorCode(ErrorCode.MissingArgument).revert();
    return;
  }

  let amount = fromBytesU64(amountBytes);
  if (GetLastError() != BytesreprError.Ok) {
    Error.fromErrorCode(ErrorCode.InvalidArgument).revert();
    return;
  }

  let amount512 = U512.fromU64(amount);

  let transferRet = transferToAccount(accountBytes, amount512);
  if (transferRet === null) {
    Error.fromErrorCode(ErrorCode.Transfer).revert();
    return;
  }
}
