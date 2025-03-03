use super::card::Card;
use super::hand::Hand;
use rand::{rngs::OsRng, Rng};
use std::collections::BTreeMap;

pub struct Table {
    rng: OsRng,
    pub deck: Vec<Card>,
    pub dealer: Hand,
    pub players: BTreeMap<String, Hand>,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            rng: OsRng,
            deck: vec![],
            dealer: Default::default(),
            players: Default::default(),
        }
    }
}

impl Table {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn draw(&mut self) -> Card {
        let card_index = self.rng.gen_range(0..self.deck.len());
        self.deck.remove(card_index)
    }

    pub fn deal_to_dealer(&mut self) {
        let card = self.draw();
        self.dealer.insert(card);
    }

    pub fn deal(&mut self, player: &str) {
        if self.deck.is_empty() {
            return;
        }

        let card = self.draw();
        if let Some(p) = self.players.get_mut(player) {
            p.insert(card);
        } else {
            self.deck.push(card);
        }
    }

    /// Retunrs -1 for dealer, 0 for draw, 1 for player
    pub fn settle(dealer: &Hand, player: &Hand) -> i8 {
        if dealer.value() == player.value() {
            return 0;
        }

        if dealer.value() == 21 {
            return -1;
        }

        if player.value() == 21 {
            return 1;
        }

        if dealer.value() < 21 && dealer.value() > player.value() {
            return -1;
        }

        1
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sanity_check() {
        let table = Table::new();
        assert_eq!(table.dealer.value(), 0);
        assert_eq!(table.deck.len(), 0);
        assert_eq!(table.players.len(), 0);
    }

    #[test]
    fn test_deal() {
        let mut table = Table::new();
        table.deck.push(Card::Two);
        table.deck.push(Card::Two);

        table.players.insert("Player A".to_string(), Hand::new());
        assert_eq!(table.players.get("Player A").unwrap().value(), 0);

        table.deal("Player A");
        assert_eq!(table.players.get("Player A").unwrap().value(), 2);

        table.deal("Player A");
        assert_eq!(table.players.get("Player A").unwrap().value(), 4);
    }

    #[test]
    fn test_settle_draw() {
        let dealer = Hand::new();
        let player = Hand::new();

        assert_eq!(Table::settle(&dealer, &player), 0);
    }

    #[test]
    fn test_settle_player_wins_21() {
        let dealer = Hand::new();
        let mut player = Hand::new();
        player.insert(Card::Ace);
        player.insert(Card::Queen);

        assert_eq!(Table::settle(&dealer, &player), 1);
    }

    #[test]
    fn test_settle_dealer_wins_21() {
        let mut dealer = Hand::new();
        let player = Hand::new();
        dealer.insert(Card::Ace);
        dealer.insert(Card::Queen);

        assert_eq!(Table::settle(&dealer, &player), -1);
    }

    #[test]
    fn test_settle_dealer_wins_not_21() {
        let mut dealer = Hand::new();
        let mut player = Hand::new();
        dealer.insert(Card::Ace);
        dealer.insert(Card::Seven);

        assert_eq!(Table::settle(&dealer, &player), -1);

        player.insert(Card::Two);
        player.insert(Card::Three);
        assert_eq!(Table::settle(&dealer, &player), -1);
    }

    #[test]
    fn test_settle_player_wins_not_21() {
        let mut dealer = Hand::new();
        dealer.insert(Card::Ace);
        dealer.insert(Card::Seven);

        let mut player = Hand::new();
        player.insert(Card::Ace);
        player.insert(Card::Eight);

        assert_eq!(Table::settle(&dealer, &player), 1);
    }
}
