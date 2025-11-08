# Thin wrapper to import the native extension when present.
# We deliberately do not import this at package import time to avoid
# forcing a build during quick Python-only work.


from .volume import volume_from_halfspaces


def try_import_native():
    try:
        import viterbo_native as _native  # built via crates/viterbo-py with maturin

        return _native
    except Exception:  # pragma: no cover - optional
        return None


__all__ = ["try_import_native", "volume_from_halfspaces"]
