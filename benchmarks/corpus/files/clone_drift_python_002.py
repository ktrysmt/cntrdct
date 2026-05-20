"""Synthetic fixture: HTTP-handler family with one drifted variant."""
# Source: https://github.com/ktrysmt/cntrdct/blob/master/benchmarks/corpus/files/clone_drift_python_002.py

def handle_a(req):
    try:
        body = req.body
        return {"status": 200, "body": body}
    except Exception:
        return {"status": 500}


def handle_b(req):
    try:
        body = req.body
        return {"status": 200, "body": body}
    except Exception:
        return {"status": 500}


def handle_c(req):
    try:
        body = req.body
        return {"status": 200, "body": body}
    except Exception:
        return {"status": 500}


def handle_d(req):
    try:
        body = req.body
        return {"status": 200, "body": body}
    except Exception:
        return {"status": 500}


def handle_drifted(req):
    try:
        body = req.body
        return {"status": 200, "body": body, "ts": 0}
    except Exception:
        return {"status": 500}
