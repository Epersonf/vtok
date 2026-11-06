import pytest
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
