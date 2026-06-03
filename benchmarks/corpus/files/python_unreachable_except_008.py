def compute(n):
    try:
        return divide(n)
    except ArithmeticError:
        return None
    except ZeroDivisionError:
        return 0
