use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug, JsonSchema)]
pub enum Hand {
    Rock = 1,
    Paper,
    Scissors,
}

impl Hand {
    fn u8(&self) -> u8 {
        *self as u8
    }

    fn compete(&self, opponent: &Hand) -> MatchResult {
        if self.u8() == opponent.u8() {
            MatchResult::Draw
        } else if (self.eq(&Hand::Rock) && opponent.eq(&Hand::Scissors))
            || (self.u8() > opponent.u8()
                && !(self.eq(&Hand::Scissors) && opponent.eq(&Hand::Rock)))
        {
            MatchResult::Win
        } else {
            MatchResult::Lose
        }
    }
}

impl From<&u8> for Hand {
    fn from(original: &u8) -> Hand {
        match original {
            1 => Hand::Rock,
            2 => Hand::Paper,
            3 => Hand::Scissors,
            _ => panic!("faild to convert into Hand({})", original),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, JsonSchema)]
pub struct Hands(Vec<Hand>);

impl Hands {
    pub fn compete(&self, opponent: &Hands, draw_point: u8) -> MatchResult {
        let mut point: u8 = 0;
        let my_hands: Vec<Hand> = self.into();
        let opponent_hands: Vec<Hand> = opponent.into();
        for (i, my_hand) in my_hands.iter().enumerate() {
            let result = my_hand.compete(&opponent_hands[i]);
            point += result.u8();
            if point > draw_point {
                return MatchResult::Win;
            }
        }
        if point == draw_point {
            MatchResult::Draw
        } else {
            MatchResult::Lose
        }
    }

    pub fn to_u8_vec(&self) -> Vec<u8> {
        let mut result: Vec<u8> = vec![];
        let hands: Vec<Hand> = self.into();
        for hand in hands.iter() {
            result.push(hand.u8());
        }
        result
    }
}

impl From<Vec<Hand>> for Hands {
    fn from(original: Vec<Hand>) -> Self {
        Self(original)
    }
}

impl From<Vec<u8>> for Hands {
    fn from(original: Vec<u8>) -> Self {
        let mut hands: Vec<Hand> = vec![];
        for num in original.iter() {
            hands.push(num.into());
        }
        hands.into()
    }
}

impl From<&Hands> for Vec<Hand> {
    fn from(original: &Hands) -> Self {
        (*original.0).to_vec()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MatchResult {
    Lose = 0,
    Draw,
    Win,
}

impl MatchResult {
    fn u8(&self) -> u8 {
        *self as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_compete() {
        let rock = Hand::Rock;
        let paper = Hand::Paper;
        let scissors = Hand::Scissors;

        assert_eq!(MatchResult::Draw, rock.compete(&rock));
        assert_eq!(MatchResult::Draw, paper.compete(&paper));
        assert_eq!(MatchResult::Draw, scissors.compete(&scissors));

        assert_eq!(MatchResult::Win, paper.compete(&rock));
        assert_eq!(MatchResult::Win, scissors.compete(&paper));
        assert_eq!(MatchResult::Win, rock.compete(&scissors));

        assert_eq!(MatchResult::Lose, rock.compete(&paper));
        assert_eq!(MatchResult::Lose, paper.compete(&scissors));
        assert_eq!(MatchResult::Lose, scissors.compete(&rock));
    }

    #[test]
    fn hands_compete() {
        let player1: Hands = vec![Hand::Rock, Hand::Paper, Hand::Scissors, Hand::Rock].into();
        let player2: Hands = vec![Hand::Scissors, Hand::Paper, Hand::Rock, Hand::Scissors].into();

        assert_eq!(MatchResult::Draw, player1.compete(&player2, 5));
        assert_eq!(MatchResult::Win, player1.compete(&player2, 4));
        assert_eq!(MatchResult::Lose, player1.compete(&player2, 6));

        assert_eq!(MatchResult::Draw, player2.compete(&player1, 3));
        assert_eq!(MatchResult::Win, player2.compete(&player1, 2));
        assert_eq!(MatchResult::Lose, player2.compete(&player1, 4));
    }
}
