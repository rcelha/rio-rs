use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum Card {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Jack,
    Queen,
    King,
    Ace,
}

impl Card {
    pub fn value(&self) -> u8 {
        *CARD_VALUE.get(self).unwrap_or(&1)
    }
}

lazy_static! {
    static ref CARD_VALUE: HashMap<Card, u8> = {
        let mut v = HashMap::new();
        v.insert(Card::Two, 2);
        v.insert(Card::Three, 3);
        v.insert(Card::Four, 4);
        v.insert(Card::Five, 5);
        v.insert(Card::Six, 6);
        v.insert(Card::Seven, 7);
        v.insert(Card::Eight, 8);
        v.insert(Card::Nine, 9);
        v.insert(Card::Jack, 10);
        v.insert(Card::Queen, 10);
        v.insert(Card::King, 10);
        v.insert(Card::Ace, 11);
        v
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sanity_check() {
        assert_eq!(Card::Two.value(), 2);
    }
}
