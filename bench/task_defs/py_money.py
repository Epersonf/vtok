"""Task `py_money` (python, target money.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="money.py", cmd="py", instruction=
 "Add to class `Money`: a static method `parse(text)` that turns strings like '12.50 USD' or '3 EUR' into a Money (cents as int, currency upper-case; raise ValueError on malformed text), "
 "and `__add__` returning a new Money (ValueError if currencies differ). Do not change existing members.",
 src={"money.py": '''class Money:
    """Amount in minor units."""

    def __init__(self, cents, currency):
        self.cents = cents
        self.currency = currency

    def __eq__(self, other):
        return self.cents == other.cents and self.currency == other.currency

    def __repr__(self):
        return f"Money({self.cents}, {self.currency!r})"

    def format(self):
        return f"{self.cents // 100}.{self.cents % 100:02d} {self.currency}"


def zero(currency):
    return Money(0, currency)
'''},
 tests={"test_money.py": '''import pytest
from money import Money, zero

def test_parse():
    assert Money.parse("12.50 USD") == Money(1250, "USD")
    assert Money.parse("3 eur") == Money(300, "EUR")
    assert Money.parse("0.05 USD") == Money(5, "USD")

def test_parse_bad():
    for s in ("", "abc", "12.5.1 USD", "12 US", "USD 12"):
        with pytest.raises(ValueError): Money.parse(s)

def test_add():
    assert Money(100, "USD") + Money(250, "USD") == Money(350, "USD")
    assert zero("USD") + Money(1, "USD") == Money(1, "USD")
    with pytest.raises(ValueError): Money(1, "USD") + Money(1, "EUR")

def test_unchanged():
    assert Money(1250, "USD").format() == "12.50 USD"
'''},
 solution={"money.py": '''import re


class Money:
    """Amount in minor units."""

    def __init__(self, cents, currency):
        self.cents = cents
        self.currency = currency

    def __eq__(self, other):
        return self.cents == other.cents and self.currency == other.currency

    def __repr__(self):
        return f"Money({self.cents}, {self.currency!r})"

    def format(self):
        return f"{self.cents // 100}.{self.cents % 100:02d} {self.currency}"

    @staticmethod
    def parse(text):
        m = re.fullmatch(r"(\\d+)(?:\\.(\\d{1,2}))? ([A-Za-z]{3})", text)
        if not m:
            raise ValueError("bad money")
        frac = (m.group(2) or "").ljust(2, "0")
        return Money(int(m.group(1)) * 100 + int(frac or 0), m.group(3).upper())

    def __add__(self, other):
        if self.currency != other.currency:
            raise ValueError("currency mismatch")
        return Money(self.cents + other.cents, self.currency)


def zero(currency):
    return Money(0, currency)
'''})
