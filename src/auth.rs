// Copyright 2016 Claus Matzinger
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate jsonwebtoken as jwt;

use self::jwt::{encode, Header};
use self::jwt::errors::Error;

#[derive(Serialize, Deserialize)]
struct Claims {
    iss: String,
    role: String,
}


pub fn get_token<I, R, S>(issuer: I, role: R, secret: S) -> Result<String, Error>
    where I: Into<String>,
          R: Into<String>,
          S: Into<String>
{
    let claims = Claims {
        iss: issuer.into(),
        role: role.into(),
    };
    encode(&Header::default(), &claims, &secret.into().as_bytes())
}
