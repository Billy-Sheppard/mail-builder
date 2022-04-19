/*
 * Copyright Stalwart Labs Ltd. See the COPYING
 * file at the top-level directory of this distribution.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use crate::encoders::encode::rfc2047_encode;

use super::Header;

/// RFC5322 e-mail address
pub struct EmailAddress {
    pub name: Option<String>,
    pub email: String,
}

/// RFC5322 grouped e-mail addresses
pub struct GroupedAddresses {
    pub name: Option<String>,
    pub addresses: Vec<Address>,
}

/// RFC5322 address
pub enum Address {
    Address(EmailAddress),
    Group(GroupedAddresses),
    List(Vec<Address>),
}

impl Address {
    /// Create an RFC5322 e-mail address
    pub fn new_address(name: Option<impl Into<String>>, email: impl Into<String>) -> Self {
        Address::Address(EmailAddress {
            name: name.map(|v| v.into()),
            email: email.into(),
        })
    }

    /// Create an RFC5322 grouped e-mail address
    pub fn new_group(name: Option<impl Into<String>>, addresses: Vec<Address>) -> Self {
        Address::Group(GroupedAddresses {
            name: name.map(|v| v.into()),
            addresses,
        })
    }

    /// Create an address list
    pub fn new_list(items: Vec<Address>) -> Self {
        Address::List(items)
    }

    pub fn unwrap_address(&self) -> &EmailAddress {
        match self {
            Address::Address(address) => address,
            _ => panic!("Address is not an EmailAddress"),
        }
    }
}

impl<'x> From<(&'x str, &'x str)> for Address {
    fn from(value: (&'x str, &'x str)) -> Self {
        Address::Address(EmailAddress {
            name: Some(value.0.into()),
            email: value.1.into(),
        })
    }
}

impl From<(String, String)> for Address {
    fn from(value: (String, String)) -> Self {
        Address::Address(EmailAddress {
            name: Some(value.0),
            email: value.1,
        })
    }
}

impl<'x> From<&'x str> for Address {
    fn from(value: &'x str) -> Self {
        Address::Address(EmailAddress {
            name: None,
            email: value.into(),
        })
    }
}

impl From<String> for Address {
    fn from(value: String) -> Self {
        Address::Address(EmailAddress {
            name: None,
            email: value,
        })
    }
}

impl<'x, T> From<Vec<T>> for Address
where
    T: Into<Address>,
{
    fn from(value: Vec<T>) -> Self {
        Address::new_list(value.into_iter().map(|x| x.into()).collect())
    }
}

impl<'x, T, U> From<(U, Vec<T>)> for Address
where
    T: Into<Address>,
    U: Into<String>,
{
    fn from(value: (U, Vec<T>)) -> Self {
        Address::Group(GroupedAddresses {
            name: Some(value.0.into()),
            addresses: value.1.into_iter().map(|x| x.into()).collect(),
        })
    }
}

impl Header for Address {
    fn write_header(
        &self,
        mut output: impl std::io::Write,
        mut bytes_written: usize,
    ) -> std::io::Result<usize> {
        match self {
            Address::Address(address) => {
                address.write_header(&mut output, bytes_written)?;
            }
            Address::Group(group) => {
                group.write_header(&mut output, bytes_written)?;
            }
            Address::List(list) => {
                for (pos, address) in list.iter().enumerate() {
                    if bytes_written
                        + (match address {
                            Address::Address(address) => {
                                address.email.len()
                                    + address.name.as_ref().map_or(0, |n| n.len() + 3)
                                    + 2
                            }
                            Address::Group(group) => {
                                group.name.as_ref().map_or(0, |name| name.len() + 2)
                            }
                            Address::List(_) => 0,
                        })
                        >= 76
                    {
                        output.write_all(b"\r\n\t")?;
                        bytes_written = 1;
                    }

                    match address {
                        Address::Address(address) => {
                            bytes_written += address.write_header(&mut output, bytes_written)?;
                            if pos < list.len() - 1 {
                                output.write_all(b", ")?;
                                bytes_written += 1;
                            }
                        }
                        Address::Group(group) => {
                            bytes_written += group.write_header(&mut output, bytes_written)?;
                            if pos < list.len() - 1 {
                                output.write_all(b"; ")?;
                                bytes_written += 1;
                            }
                        }
                        Address::List(_) => unreachable!(),
                    }
                }
            }
        }
        output.write_all(b"\r\n")?;
        Ok(0)
    }
}

impl Header for EmailAddress {
    fn write_header(
        &self,
        mut output: impl std::io::Write,
        mut bytes_written: usize,
    ) -> std::io::Result<usize> {
        if let Some(name) = &self.name {
            bytes_written += rfc2047_encode(name, &mut output)?;
            if bytes_written + self.email.len() + 2 >= 76 {
                output.write_all(b"\r\n\t")?;
                bytes_written = 1;
            } else {
                output.write_all(b" ")?;
                bytes_written += 1;
            }
        }

        output.write_all(b"<")?;
        output.write_all(self.email.as_bytes())?;
        output.write_all(b">")?;

        Ok(bytes_written + self.email.len() + 2)
    }
}

impl Header for GroupedAddresses {
    fn write_header(
        &self,
        mut output: impl std::io::Write,
        mut bytes_written: usize,
    ) -> std::io::Result<usize> {
        if let Some(name) = &self.name {
            bytes_written += rfc2047_encode(name, &mut output)? + 2;
            output.write_all(b": ")?;
        }

        for (pos, address) in self.addresses.iter().enumerate() {
            let address = address.unwrap_address();

            if bytes_written
                + address.email.len()
                + address.name.as_ref().map_or(0, |n| n.len() + 3)
                + 2
                >= 76
            {
                output.write_all(b"\r\n\t")?;
                bytes_written = 1;
            }

            bytes_written += address.write_header(&mut output, bytes_written)?;
            if pos < self.addresses.len() - 1 {
                output.write_all(b", ")?;
                bytes_written += 2;
            }
        }

        Ok(bytes_written)
    }
}
