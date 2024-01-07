use soroban_sdk::{Vec, String};

use crate::storage::Swap;

pub fn verify_swap(operations: &Vec<Swap>) {
    for (current, next) in operations.iter().zip(operations.iter().skip(1)) {
        if current.ask_asset != next.offer_asset {
            panic!("Multihop: Swap: Provided bad swap order");
        }
    }
}

pub fn verify_reverse_swap(operations: &Vec<Swap>) {
    for (current, next) in operations.iter().zip(operations.iter().skip(1)) {
        if current.offer_asset != next.ask_asset {
            panic!("Multihop: Reverse swap: Provided bad swap order");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{storage::Swap, utils::verify_swap};

    use soroban_sdk::{testutils::Address as _, vec, Address, Env};

    #[test]
    fn verify_operations_in_swap_should_work() {
        let env = Env::default();

        let token1 = Address::from_string(&String::from_str(&env, "CDF2CJ26YHWB37JYTUFS722WJBJPG5OP5XN4TGZZPCEEUWHEIMGQG2O3"));
        let token2 = Address::from_string(&String::from_str(&env, "CB64D3G7SM2RTH6JSGG34DDTFTQ5CFDKVDZJZSODMCX4NJ2HV2KN7OHT"));
        let token3 = Address::from_string(&String::from_str(&env, "CAM3XZFCVAG6KJQUIAW2YWCGZQJ6CR6QIAQ5MAWU7GMM4ZZZCJ7JVDSH"));
        let token4 = Address::from_string(&String::from_str(&env, "CBXBKAB6QIRUGTG77OQZHC46BIIPA5WDKIKZKPA2H7Q7CPKQ555W3EVB"));

        let swap1 = Swap {
            offer_asset: token1.clone(),
            ask_asset: token2.clone(),
        };
        let swap2 = Swap {
            offer_asset: token2.clone(),
            ask_asset: token3.clone(),
        };
        let swap3 = Swap {
            offer_asset: token3.clone(),
            ask_asset: token4.clone(),
        };

        let operations = vec![&env, swap1, swap2, swap3];

        verify_swap(&operations);
    }

    #[test]
    fn verify_operations_in_reverse_swap_should_work() {
        let env = Env::default();

        let token1 = Address::from_string(&String::from_str(&env, "CDF2CJ26YHWB37JYTUFS722WJBJPG5OP5XN4TGZZPCEEUWHEIMGQG2O3"));
        let token2 = Address::from_string(&String::from_str(&env, "CB64D3G7SM2RTH6JSGG34DDTFTQ5CFDKVDZJZSODMCX4NJ2HV2KN7OHT"));
        let token3 = Address::from_string(&String::from_str(&env, "CAM3XZFCVAG6KJQUIAW2YWCGZQJ6CR6QIAQ5MAWU7GMM4ZZZCJ7JVDSH"));
        let token4 = Address::from_string(&String::from_str(&env, "CBXBKAB6QIRUGTG77OQZHC46BIIPA5WDKIKZKPA2H7Q7CPKQ555W3EVB"));

        let swap1 = Swap {
            offer_asset: token3.clone(),
            ask_asset: token4.clone(),
        };
        let swap2 = Swap {
            offer_asset: token2.clone(),
            ask_asset: token3.clone(),
        };
        let swap3 = Swap {
            offer_asset: token1.clone(),
            ask_asset: token2.clone(),
        };

        let operations = vec![&env, swap1, swap2, swap3];

        verify_reverse_swap(&operations);
    }

    #[test]
    #[should_panic(expected = "Multihop: Swap: Provided bad swap order")]
    fn verify_operations_should_fail_when_bad_order_provided() {
        let env = Env::default();

        let token1 = Address::from_string(&String::from_str(&env, "CDF2CJ26YHWB37JYTUFS722WJBJPG5OP5XN4TGZZPCEEUWHEIMGQG2O3"));
        let token2 = Address::from_string(&String::from_str(&env, "CB64D3G7SM2RTH6JSGG34DDTFTQ5CFDKVDZJZSODMCX4NJ2HV2KN7OHT"));
        let token3 = Address::from_string(&String::from_str(&env, "CAM3XZFCVAG6KJQUIAW2YWCGZQJ6CR6QIAQ5MAWU7GMM4ZZZCJ7JVDSH"));
        let token4 = Address::from_string(&String::from_str(&env, "CBXBKAB6QIRUGTG77OQZHC46BIIPA5WDKIKZKPA2H7Q7CPKQ555W3EVB"));

        let swap1 = Swap {
            offer_asset: token1.clone(),
            ask_asset: token2.clone(),
        };
        let swap2 = Swap {
            offer_asset: token3.clone(),
            ask_asset: token4.clone(),
        };

        let operations = vec![&env, swap1, swap2];

        verify_swap(&operations);
    }

    #[test]
    #[should_panic(expected = "Multihop: Reverse swap: Provided bad swap order")]
    fn verify_operations_reverse_swap_should_fail_when_bad_order_provided() {
        let env = Env::default();

        let token1 = Address::from_string(&String::from_str(&env, "CDF2CJ26YHWB37JYTUFS722WJBJPG5OP5XN4TGZZPCEEUWHEIMGQG2O3"));
        let token2 = Address::from_string(&String::from_str(&env, "CB64D3G7SM2RTH6JSGG34DDTFTQ5CFDKVDZJZSODMCX4NJ2HV2KN7OHT"));
        let token3 = Address::from_string(&String::from_str(&env, "CAM3XZFCVAG6KJQUIAW2YWCGZQJ6CR6QIAQ5MAWU7GMM4ZZZCJ7JVDSH"));
        let token4 = Address::from_string(&String::from_str(&env, "CBXBKAB6QIRUGTG77OQZHC46BIIPA5WDKIKZKPA2H7Q7CPKQ555W3EVB"));

        let swap1 = Swap {
            offer_asset: token1.clone(),
            ask_asset: token2.clone(),
        };
        let swap2 = Swap {
            offer_asset: token3.clone(),
            ask_asset: token4.clone(),
        };

        let operations = vec![&env, swap1, swap2];

        verify_reverse_swap(&operations);
    }
}
