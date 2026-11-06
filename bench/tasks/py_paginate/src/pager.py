"""Pagination helpers."""
# NOTE: pages are 1-based

def paginate(items, page, size):
    """Return one page of items."""
    start = page * size
    end = start + size
    return list(items[start:end])


def page_count(total, size):
    """Number of pages needed for `total` items."""
    return (total + size - 1) // size
