package check

func classify(x int) string {
	if x > 0 {
		return "positive"
	} else {
		panic("non-positive")
	}
}
