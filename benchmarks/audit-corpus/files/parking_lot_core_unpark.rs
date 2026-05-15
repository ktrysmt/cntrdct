// Source: https://github.com/Amanieu/parking_lot/blob/d7828fff7b5d6327ae608e82db45f888b344449a/core/src/parking_lot.rs
// License: MIT OR Apache-2.0
// Note: minimal extract of two top-level `pub unsafe fn` items from Amanieu/parking_lot@d7828fff7b5d6327ae608e82db45f888b344449a core/src/parking_lot.rs (parking_lot_core-v0.9.12 release tag, MIT OR Apache-2.0). `pub unsafe fn unpark_one` at upstream line 732 (corpus line 29) and `pub unsafe fn unpark_requeue` at upstream line 888 (corpus line 122) each carry a `///` doc block ending in `must not panic or call into any function in `parking_lot`.` The doc claim of "must not panic" trips cntrdct's Pattern B `doc_lc.contains("panic")` substring check, but neither function body contains any of cntrdct's Pattern B markers (`panic!`, `unwrap`, `expect(`, `unreachable!`, `assert!`, `assert_eq!`, `assert_ne!`, `todo!`, `unimplemented!`, `debug_assert`), so spec F4 fires — the textbook Tan SOSP 2007 §3.2 Pattern B ("bad comment": panic claim without panicking constructs in the body) bug shape, the same one cntrdct's batch-14 `zarrs/zarrs@3b944c57` `round_bytes_*` family flags on the panic-mismatch direction. The doc here is a contract for callbacks rather than a panic-prone implementation, but cntrdct's syntactic substring rule does not distinguish 'must not panic' (callback contract) from 'panics if' (implementation claim) — both are spec F4 hits on the same trigger. Diversifies `comment-code`'s Pattern B audit evidence from a single upstream (zarrs Zarr-format data-type bindings, 6 entries) to two upstreams (zarrs 6 + parking_lot_core synchronization primitives 2), reducing the regression-detection risk of single-upstream dominance the way batches 11 / 12 / 13 / 14 / 15 progressively did for Pattern C and Pattern A. The intervening `pub unsafe fn unpark_all` (upstream lines 797-858) is dropped from the extract because cntrdct's per-fn loop only inspects `///` doc blocks immediately preceding `function_item` and unpark_all's own Pattern B status is orthogonal to this batch's scope; tree-sitter parses unresolved identifiers (`UnparkResult`, `UnparkToken`, `lock_bucket`, `lock_bucket_pair`, `unlock_bucket_pair`, `ThreadData`, `RequeueOp`, `Ordering`, `ptr::null`) syntactically without requiring resolution. SHA-256 is of the audit-corpus file as committed (minimal extract, per `benchmarks/audit-corpus/README.md` `Per-detector seed targets` item 4).

/// Unparks one thread from the queue associated with the given key.
///
/// The `callback` function is called while the queue is locked and before the
/// target thread is woken up. The `UnparkResult` argument to the function
/// indicates whether a thread was found in the queue and whether this was the
/// last thread in the queue. This value is also returned by `unpark_one`.
///
/// The `callback` function should return an `UnparkToken` value which will be
/// passed to the thread that is unparked. If no thread is unparked then the
/// returned value is ignored.
///
/// # Safety
///
/// You should only call this function with an address that you control, since
/// you could otherwise interfere with the operation of other synchronization
/// primitives.
///
/// The `callback` function is called while the queue is locked and must not
/// panic or call into any function in `parking_lot`.
///
/// The `parking_lot` functions are not re-entrant and calling this method
/// from the context of an asynchronous signal handler may result in undefined
/// behavior, including corruption of internal state and/or deadlocks.
#[inline]
pub unsafe fn unpark_one(
    key: usize,
    callback: impl FnOnce(UnparkResult) -> UnparkToken,
) -> UnparkResult {
    // Lock the bucket for the given key
    let bucket = lock_bucket(key);

    // Find a thread with a matching key and remove it from the queue
    let mut link = &bucket.queue_head;
    let mut current = bucket.queue_head.get();
    let mut previous = ptr::null();
    let mut result = UnparkResult::default();
    while !current.is_null() {
        if (*current).key.load(Ordering::Relaxed) == key {
            // Remove the thread from the queue
            let next = (*current).next_in_queue.get();
            link.set(next);
            if bucket.queue_tail.get() == current {
                bucket.queue_tail.set(previous);
            } else {
                // Scan the rest of the queue to see if there are any other
                // entries with the given key.
                let mut scan = next;
                while !scan.is_null() {
                    if (*scan).key.load(Ordering::Relaxed) == key {
                        result.have_more_threads = true;
                        break;
                    }
                    scan = (*scan).next_in_queue.get();
                }
            }

            // Invoke the callback before waking up the thread
            result.unparked_threads = 1;
            result.be_fair = (*bucket.fair_timeout.get()).should_timeout();
            let token = callback(result);

            // Set the token for the target thread
            (*current).unpark_token.set(token);

            // This is a bit tricky: we first lock the ThreadParker to prevent
            // the thread from exiting and freeing its ThreadData if its wait
            // times out. Then we unlock the queue since we don't want to keep
            // the queue locked while we perform a system call. Finally we wake
            // up the parked thread.
            let handle = (*current).parker.unpark_lock();
            // SAFETY: We hold the lock here, as required
            bucket.mutex.unlock();
            handle.unpark();

            return result;
        } else {
            link = &(*current).next_in_queue;
            previous = current;
            current = link.get();
        }
    }

    // No threads with a matching key were found in the bucket
    callback(result);
    // SAFETY: We hold the lock here, as required
    bucket.mutex.unlock();
    result
}

/// Removes all threads from the queue associated with `key_from`, optionally
/// unparks the first one and requeues the rest onto the queue associated with
/// `key_to`.
///
/// The `validate` function is called while both queues are locked. Its return
/// value will determine which operation is performed, or whether the operation
/// should be aborted. See `RequeueOp` for details about the different possible
/// return values.
///
/// The `callback` function is also called while both queues are locked. It is
/// passed the `RequeueOp` returned by `validate` and an `UnparkResult`
/// indicating whether a thread was unparked and whether there are threads still
/// parked in the new queue. This `UnparkResult` value is also returned by
/// `unpark_requeue`.
///
/// The `callback` function should return an `UnparkToken` value which will be
/// passed to the thread that is unparked. If no thread is unparked then the
/// returned value is ignored.
///
/// # Safety
///
/// You should only call this function with an address that you control, since
/// you could otherwise interfere with the operation of other synchronization
/// primitives.
///
/// The `validate` and `callback` functions are called while the queue is locked
/// and must not panic or call into any function in `parking_lot`.
#[inline]
pub unsafe fn unpark_requeue(
    key_from: usize,
    key_to: usize,
    validate: impl FnOnce() -> RequeueOp,
    callback: impl FnOnce(RequeueOp, UnparkResult) -> UnparkToken,
) -> UnparkResult {
    // Lock the two buckets for the given key
    let (bucket_from, bucket_to) = lock_bucket_pair(key_from, key_to);

    // If the validation function fails, just return
    let mut result = UnparkResult::default();
    let op = validate();
    if op == RequeueOp::Abort {
        // SAFETY: Both buckets are locked, as required.
        unlock_bucket_pair(bucket_from, bucket_to);
        return result;
    }

    // Remove all threads with the given key in the source bucket
    let mut link = &bucket_from.queue_head;
    let mut current = bucket_from.queue_head.get();
    let mut previous = ptr::null();
    let mut requeue_threads: *const ThreadData = ptr::null();
    let mut requeue_threads_tail: *const ThreadData = ptr::null();
    let mut wakeup_thread = None;
    while !current.is_null() {
        if (*current).key.load(Ordering::Relaxed) == key_from {
            // Remove the thread from the queue
            let next = (*current).next_in_queue.get();
            link.set(next);
            if bucket_from.queue_tail.get() == current {
                bucket_from.queue_tail.set(previous);
            }

            // Prepare the first thread for wakeup and requeue the rest.
            if (op == RequeueOp::UnparkOneRequeueRest || op == RequeueOp::UnparkOne)
                && wakeup_thread.is_none()
            {
                wakeup_thread = Some(current);
                result.unparked_threads = 1;
            } else {
                if !requeue_threads.is_null() {
                    (*requeue_threads_tail).next_in_queue.set(current);
                } else {
                    requeue_threads = current;
                }
                requeue_threads_tail = current;
                (*current).key.store(key_to, Ordering::Relaxed);
                result.requeued_threads += 1;
            }
            if op == RequeueOp::UnparkOne || op == RequeueOp::RequeueOne {
                // Scan the rest of the queue to see if there are any other
                // entries with the given key.
                let mut scan = next;
                while !scan.is_null() {
                    if (*scan).key.load(Ordering::Relaxed) == key_from {
                        result.have_more_threads = true;
                        break;
                    }
                    scan = (*scan).next_in_queue.get();
                }
                break;
            }
            current = next;
        } else {
            link = &(*current).next_in_queue;
            previous = current;
            current = link.get();
        }
    }

    // Add the requeued threads to the destination bucket
    if !requeue_threads.is_null() {
        (*requeue_threads_tail).next_in_queue.set(ptr::null());
        if !bucket_to.queue_head.get().is_null() {
            (*bucket_to.queue_tail.get())
                .next_in_queue
                .set(requeue_threads);
        } else {
            bucket_to.queue_head.set(requeue_threads);
        }
        bucket_to.queue_tail.set(requeue_threads_tail);
    }

    // Invoke the callback before waking up the thread
    if result.unparked_threads != 0 {
        result.be_fair = (*bucket_from.fair_timeout.get()).should_timeout();
    }
    let token = callback(op, result);

    // See comment in unpark_one for why we mess with the locking
    if let Some(wakeup_thread) = wakeup_thread {
        (*wakeup_thread).unpark_token.set(token);
        let handle = (*wakeup_thread).parker.unpark_lock();
        // SAFETY: Both buckets are locked, as required.
        unlock_bucket_pair(bucket_from, bucket_to);
        handle.unpark();
    } else {
        // SAFETY: Both buckets are locked, as required.
        unlock_bucket_pair(bucket_from, bucket_to);
    }

    result
}
