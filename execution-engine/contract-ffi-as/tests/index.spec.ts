import "assemblyscript/std/portable";
import {URef, decodeOptional, toBytesU32, serializeArguments, Key, toBytesString, CLValue } from "../assembly";

import test from "ava";
import {hex2bin} from "./utils";

test('decoded optional is some', t => {
    let optionalSome = new Uint8Array(10);
    for (let i = 0; i < 10; i++) {
        optionalSome[i] = i + 1;
    }
    let res = decodeOptional(optionalSome);
    t.not(res, null);
    t.deepEqual(Array.from(res), [2, 3, 4, 5, 6, 7, 8, 9, 10]);
});

test('decoded optional is none', t => {
    let optionalSome = new Uint8Array(10);
    optionalSome[0] = 0;
    let res = decodeOptional(optionalSome);
    t.is(res, null);
});

test("decode uref from bytes with access rights", t => {
    const truth = hex2bin("2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a0107");
    let uref = URef.fromBytes(truth);
    t.not(uref, null);
    t.deepEqual(Array.from(uref.getBytes()), [
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42,
    ]);
    t.is(uref.getAccessRights(), 0x07); // NOTE: 0x07 is READ_ADD_WRITE
})

test("decode uref from bytes without access rights", t => {
    const truth = hex2bin("2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a00");
    let uref = URef.fromBytes(truth);
    t.not(uref, null);
    t.deepEqual(Array.from(uref.getBytes()), [
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42,
    ]);
    t.is(uref.getAccessRights(), null);
    let serialized = uref.toBytes();
    t.deepEqual(Array.from(serialized), Array.from(truth));
})

test("serializes u32", t => {
    const truth = [239, 190, 173, 222];
    let res = toBytesU32(3735928559);
    t.deepEqual(Array.from(res), truth);
})

test("serialize string", t => {
    // Rust: let bytes = "hello_world".to_bytes().unwrap();
    const truth = hex2bin("0b00000068656c6c6f5f776f726c64");
    t.deepEqual(toBytesString("hello_world"), Array.from(truth));
})

test("key of uref variant serializes", t => {
    // URef with access rights
    const truth = hex2bin("022a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a0107");
    const urefBytes = hex2bin("2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a");
    let uref = new URef(urefBytes, 0x07);
    let key = Key.fromURef(uref);
    let serialized = key.toBytes();
    t.deepEqual(Array.from(serialized), Array.from(truth));
});

test("serialize args", t => {
    // let args = ("get_payment_purse",).parse().unwrap().to_bytes().unwrap();
    const truth = hex2bin("0100000015000000110000006765745f7061796d656e745f70757273650a");
    let serialized = serializeArguments([
        CLValue.fromString("get_payment_purse"),
    ]);
    t.deepEqual(Array.from(serialized), Array.from(truth));
})
