mod partial_tries {
    use engine_shared::newtypes::CorrelationId;

    use crate::{
        transaction_source::{Transaction, TransactionSource},
        trie::Trie,
        trie_store::operations::{
            self,
            tests::{
                InMemoryTestContext, LmdbTestContext, TestKey, TestValue, TEST_LEAVES,
                TEST_TRIE_GENERATORS,
            },
        },
    };

    #[test]
    fn lmdb_keys_from_n_leaf_partial_trie_had_expected_results() {
        for (num_leaves, generator) in TEST_TRIE_GENERATORS.iter().enumerate() {
            let correlation_id = CorrelationId::new();
            let (root_hash, tries) = generator().unwrap();
            let context = LmdbTestContext::new(&tries).unwrap();
            let test_leaves = TEST_LEAVES;
            let (used, _) = test_leaves.split_at(num_leaves);

            let expected = {
                let mut tmp = used
                    .iter()
                    .filter_map(Trie::key)
                    .cloned()
                    .collect::<Vec<TestKey>>();
                tmp.sort();
                tmp
            };
            let actual = {
                let txn = context.environment.create_read_txn().unwrap();
                let mut tmp = operations::keys::<TestKey, TestValue, _, _>(
                    correlation_id,
                    &txn,
                    &context.store,
                    &root_hash,
                )
                .filter_map(Result::ok)
                .collect::<Vec<TestKey>>();
                txn.commit().unwrap();
                tmp.sort();
                tmp
            };
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn in_memory_keys_from_n_leaf_partial_trie_had_expected_results() {
        for (num_leaves, generator) in TEST_TRIE_GENERATORS.iter().enumerate() {
            let correlation_id = CorrelationId::new();
            let (root_hash, tries) = generator().unwrap();
            let context = InMemoryTestContext::new(&tries).unwrap();
            let test_leaves = TEST_LEAVES;
            let (used, _) = test_leaves.split_at(num_leaves);

            let expected = {
                let mut tmp = used
                    .iter()
                    .filter_map(Trie::key)
                    .cloned()
                    .collect::<Vec<TestKey>>();
                tmp.sort();
                tmp
            };
            let actual = {
                let txn = context.environment.create_read_txn().unwrap();
                let mut tmp = operations::keys::<TestKey, TestValue, _, _>(
                    correlation_id,
                    &txn,
                    &context.store,
                    &root_hash,
                )
                .filter_map(Result::ok)
                .collect::<Vec<TestKey>>();
                txn.commit().unwrap();
                tmp.sort();
                tmp
            };
            assert_eq!(actual, expected);
        }
    }
}

mod full_tries {
    use engine_shared::newtypes::{Blake2bHash, CorrelationId};

    use crate::{
        transaction_source::{Transaction, TransactionSource},
        trie::Trie,
        trie_store::operations::{
            self,
            tests::{
                InMemoryTestContext, TestKey, TestValue, EMPTY_HASHED_TEST_TRIES, TEST_LEAVES,
                TEST_TRIE_GENERATORS,
            },
        },
    };

    #[test]
    fn in_memory_keys_from_n_leaf_full_trie_had_expected_results() {
        let correlation_id = CorrelationId::new();
        let context = InMemoryTestContext::new(EMPTY_HASHED_TEST_TRIES).unwrap();
        let mut states: Vec<Blake2bHash> = Vec::new();

        for (state_index, generator) in TEST_TRIE_GENERATORS.iter().enumerate() {
            let (root_hash, tries) = generator().unwrap();
            context.update(&tries).unwrap();
            states.push(root_hash);

            for (num_leaves, state) in states[..state_index].iter().enumerate() {
                let test_leaves = TEST_LEAVES;
                let (used, _unused) = test_leaves.split_at(num_leaves);

                let expected = {
                    let mut tmp = used
                        .iter()
                        .filter_map(Trie::key)
                        .cloned()
                        .collect::<Vec<TestKey>>();
                    tmp.sort();
                    tmp
                };
                let actual = {
                    let txn = context.environment.create_read_txn().unwrap();
                    let mut tmp = operations::keys::<TestKey, TestValue, _, _>(
                        correlation_id,
                        &txn,
                        &context.store,
                        &state,
                    )
                    .filter_map(Result::ok)
                    .collect::<Vec<TestKey>>();
                    txn.commit().unwrap();
                    tmp.sort();
                    tmp
                };
                assert_eq!(actual, expected);
            }
        }
    }
}

#[cfg(debug_assertions)]
mod keys_iterator {
    use engine_shared::newtypes::{Blake2bHash, CorrelationId};
    use types::bytesrepr;

    use crate::{
        transaction_source::TransactionSource,
        trie::{Pointer, Trie},
        trie_store::operations::{
            self,
            tests::{
                hash_test_tries, HashedTestTrie, HashedTrie, InMemoryTestContext, TestKey,
                TestValue, TEST_LEAVES,
            },
        },
    };

    fn create_invalid_extension_trie(
    ) -> Result<(Blake2bHash, Vec<HashedTestTrie>), bytesrepr::Error> {
        let leaves = hash_test_tries(&TEST_LEAVES[2..3])?;
        let ext_1 = HashedTrie::new(Trie::extension(
            vec![0u8, 0],
            Pointer::NodePointer(leaves[0].hash),
        ))?;

        let root = HashedTrie::new(Trie::node(&[(0, Pointer::NodePointer(ext_1.hash))]))?;
        let root_hash = root.hash;

        let tries = vec![root, ext_1, leaves[0].clone()];

        Ok((root_hash, tries))
    }

    fn create_invalid_path_trie() -> Result<(Blake2bHash, Vec<HashedTestTrie>), bytesrepr::Error> {
        let leaves = hash_test_tries(&TEST_LEAVES[..1])?;

        let root = HashedTrie::new(Trie::node(&[(1, Pointer::NodePointer(leaves[0].hash))]))?;
        let root_hash = root.hash;

        let tries = vec![root, leaves[0].clone()];

        Ok((root_hash, tries))
    }

    fn create_invalid_hash_trie() -> Result<(Blake2bHash, Vec<HashedTestTrie>), bytesrepr::Error> {
        let leaves = hash_test_tries(&TEST_LEAVES[..2])?;

        let root = HashedTrie::new(Trie::node(&[(0, Pointer::NodePointer(leaves[1].hash))]))?;
        let root_hash = root.hash;

        let tries = vec![root, leaves[0].clone()];

        Ok((root_hash, tries))
    }

    macro_rules! return_on_err {
        ($x:expr) => {
            match $x {
                Ok(result) => result,
                Err(_) => {
                    return; // we expect the test to panic, so this will cause a test failure
                }
            }
        };
    }

    fn test_trie(root_hash: Blake2bHash, tries: Vec<HashedTestTrie>) {
        let correlation_id = CorrelationId::new();
        let context = return_on_err!(InMemoryTestContext::new(&tries));
        let txn = return_on_err!(context.environment.create_read_txn());
        let _tmp = operations::keys::<TestKey, TestValue, _, _>(
            correlation_id,
            &txn,
            &context.store,
            &root_hash,
        )
        .collect::<Vec<_>>();
    }

    #[test]
    #[should_panic]
    fn should_panic_on_leaf_after_extension() {
        let (root_hash, tries) = return_on_err!(create_invalid_extension_trie());
        test_trie(root_hash, tries);
    }

    #[test]
    #[should_panic]
    fn should_panic_when_key_not_matching_path() {
        let (root_hash, tries) = return_on_err!(create_invalid_path_trie());
        test_trie(root_hash, tries);
    }

    #[test]
    #[should_panic]
    fn should_panic_on_pointer_to_nonexisting_hash() {
        let (root_hash, tries) = return_on_err!(create_invalid_hash_trie());
        test_trie(root_hash, tries);
    }
}
