package worker

// start runs a job and a deferred cleanup closure.
func start(id int) {
	job := func() {
		execute(id)
	}
	defer cleanup(id)
	job()
}

func helper() int {
	x := compute()
	return x
}
