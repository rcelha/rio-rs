#[cfg(test)]
mod test {
    use black_jack::card::*;
    use black_jack::table::*;

    fn table() -> Table {
        let mut table = Table::new();
        for _ in 0..52 {
            table.deck.push(Card::Two);
        }
        table
            .players
            .insert("Player 1".to_string(), Default::default());
        table
            .players
            .insert("Player 2".to_string(), Default::default());
        table
            .players
            .insert("Player 3".to_string(), Default::default());

        table.deal_to_dealer();
        table.deal_to_dealer();
        table.deal("Player 1");
        table.deal("Player 1");
        table.deal("Player 2");
        table.deal("Player 2");
        table.deal("Player 3");
        table.deal("Player 3");

        table
    }

    #[test]
    fn test_tie() {
        let table = table();
        let result = Table::settle(&table.dealer, &table.players.get("Player 1").unwrap());
        assert_eq!(result, 0);
    }

    #[test]
    fn test_dealer_wins() {
        let mut table = table();
        table.deal_to_dealer();
        let result = Table::settle(&table.dealer, &table.players.get("Player 1").unwrap());
        assert_eq!(result, -1);
    }

    #[test]
    fn test_player_1_wins() {
        let mut table = table();
        table.deal("Player 1");
        let result = Table::settle(&table.dealer, &table.players.get("Player 1").unwrap());
        assert_eq!(result, 1);
    }
}
