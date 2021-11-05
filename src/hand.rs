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

    fn matches(&self, opponent: &Hand) -> MatchResult {
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
    pub fn matches(self, opponent: Hands, draw_point: u8) -> MatchResult {
        let mut point: u8 = 0;
        let my_hands: Vec<Hand> = self.into();
        let opponent_hands: Vec<Hand> = opponent.into();
        for (i, my_hand) in my_hands.iter().enumerate() {
            let result = my_hand.matches(&opponent_hands[i]);
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

impl From<Hands> for Vec<Hand> {
    fn from(original: Hands) -> Vec<Hand> {
        original.0
    }
}

#[derive(Clone, Copy)]
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
