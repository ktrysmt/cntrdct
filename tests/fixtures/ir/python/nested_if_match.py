def classify(x):
    if x < 0:
        if x == -1:
            return "minus_one"
        else:
            return "negative"
    elif x == 0:
        return "zero"
    else:
        return "positive"
