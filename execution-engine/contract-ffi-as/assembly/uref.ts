import {Option} from "./option";

export const UREF_ADDR_LENGTH = 32;
export const OPTION_TAG_SERIALIZED_LENGTH = 1;
export const ACCESS_RIGHTS_SERIALIZED_LENGTH = 1;
export const UREF_SERIALIZED_LENGTH = UREF_ADDR_LENGTH + OPTION_TAG_SERIALIZED_LENGTH + ACCESS_RIGHTS_SERIALIZED_LENGTH;

export enum AccessRights{
    NONE = 0x0,
    READ = 0x1,
    WRITE = 0x2,
    READ_WRITE = 0x3,
    ADD = 0x4,
    READ_ADD = 0x5,
    ADD_WRITE = 0x6,
    READ_ADD_WRITE = 0x07,
}

export class URef {
    private bytes: Uint8Array;
    private accessRights: AccessRights

    constructor(bytes: Uint8Array, accessRights: AccessRights) {
        this.bytes = bytes;
        this.accessRights = accessRights;
    }

    public getBytes(): Uint8Array {
        return this.bytes;
    }

    public getAccessRights(): AccessRights {
        return this.accessRights;
    }

    static fromBytes(bytes: Uint8Array): URef | null {
        let urefBytes = bytes.subarray(0, UREF_ADDR_LENGTH);
        let accessRightsBytes =  Option.fromBytes(bytes.subarray(UREF_ADDR_LENGTH));
        if(accessRightsBytes.isNone())
            return new URef(urefBytes, AccessRights.NONE);

        let accessRights = <u8>(<Uint8Array>accessRightsBytes.unwrap())[0];
        return new URef(urefBytes, accessRights);
    }

    toBytes(): Array<u8> {
        let result = new Array<u8>(this.bytes.length);
        for (let i = 0; i < this.bytes.length; i++) {
            result[i] = this.bytes[i];
        }

        if (this.accessRights == AccessRights.NONE) {
            result.push(0);
            return result;
        }

        result.push(1);
        result.push(<u8>this.accessRights);
        return result;
    }

    //TODO: read():CLValue{}
}
