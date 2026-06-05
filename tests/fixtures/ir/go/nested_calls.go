package pipeline

func run(a int, b int) int {
	first(a, b)
	obj.handler.process(a)
	return combine(a, b)
}
