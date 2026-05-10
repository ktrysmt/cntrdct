fn early_return_then_dead_call() {
    return;
    do_more_work();
}

fn do_more_work() {}
