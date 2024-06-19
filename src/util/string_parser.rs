// TODO: something weird is going on and the compiling the steam-cli bin doesn't actually sees this
// function as unused wheras the discord-steam-cli sees that function as used and so we get
// warnings when compiling one and not the other
pub fn batch_string(input: &str, max_size: usize, separator: char) -> Result<Vec<&str>, Error> {
    let mut indexes_to_split_on: Vec<usize> = vec![];
    let mut current_index = 0;
    loop {
        if current_index + max_size > input.len() {
            break;
        }
        let current_section = &input[..current_index + max_size + 1];
        current_index = current_section
            .rfind(separator)
            .ok_or(Error::SeparatorNotFound)?;
        if !indexes_to_split_on.is_empty() && &current_index == indexes_to_split_on.last().unwrap()
        {
            return Err(Error::SeparatorNotFound);
        }
        indexes_to_split_on.push(current_index);
        current_index += 1;
    }
    let mut batches: Vec<_> = vec![];
    let mut current_index = 0;
    for index in indexes_to_split_on {
        batches.push(&input[current_index..index]);
        current_index = index + 1;
    }
    batches.push(&input[current_index..]);

    Ok(batches)
}

#[derive(Debug)]
pub enum Error {
    SeparatorNotFound,
}

#[cfg(test)]
mod tests {
    macro_rules! batch_string_tests {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (input, expected) = $value;
                    let (input_string, max_size, separator) = input;
                    let output = batch_string(input_string, max_size, separator).unwrap();
                    assert_eq!(output.len(), expected.len(), "lengths were different. actual:\n{:?}\nexpected:\n{:?}", output, expected);
                    for (actual, expected) in izip!(output, expected) {
                        assert_eq!(actual, expected);
                    }
                }
            )*
        }
    }

    use super::batch_string;
    use itertools::izip;

    batch_string_tests! {
        batch_string_0: (("something", 10, ' '), vec!["something"]),
        batch_string_1: (("something else", 10, ' '), vec!["something", "else"]),
        batch_string_2: (
            (
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod",
                15,
                ' ',
            ), vec!["Lorem ipsum", "dolor sit amet,", "consectetur", "adipiscing", "elit, sed do", "eiusmod",]),
    }
}
