use super::card::Card;

#[derive(Default, Debug)]
pub struct Hand(pub Vec<Card>);

impl Hand {
    pub fn value(&self) -> u8 {
        self.0
            .iter()
            .map(|x| x.value())
            .reduce(|x, acc| acc + x)
            .unwrap_or(0)
    }

    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert(&mut self, card: Card) {
        self.0.push(card)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sanity_check() {
        let mut hand = Hand::new();
        assert_eq!(hand.value(), 0);

        hand.insert(Card::Two);
        assert_eq!(hand.value(), 2);

        hand.insert(Card::Two);
        assert_eq!(hand.value(), 4);

        hand.insert(Card::Ace);
        assert_eq!(hand.value(), 15);
    }
}
