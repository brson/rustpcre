/* rustpcre - rust PCRE bindings
 *
 * Copyright 2011 Google Inc. All Rights Reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *    http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std;

import ctypes::{c_int, void};

export pcre, mk_pcre, match;

iface match {
    fn matched() -> bool;
    fn substring(index: uint) -> str;
    fn substrings() -> [str];
    fn named(name: str) -> str;
}

iface pcre {
    fn match(target: str) -> match;
}

type _pcre = *void;
type _pcre_extra = *void;

#[link_name = "pcre"]
native mod _native {
    fn pcre_compile(pattern: str::sbuf, options: c_int, errptr: *str::sbuf,
                    erroffset: *c_int, tableptr: *u8) -> *_pcre;
    fn pcre_exec(re: *_pcre, extra: *_pcre_extra, subject: str::sbuf,
                 length: c_int, startoffset: c_int, options: c_int,
                 ovector: *c_int, ovecsize: c_int) -> c_int;
    fn pcre_get_stringnumber(re: *_pcre, name: *u8) -> c_int;
    fn pcre_refcount(re: *_pcre, adj: c_int) -> c_int;
}

resource _pcre_res(re: *_pcre) {
    _native::pcre_refcount(re, -1 as c_int);
}

fn mk_match(m: option::t<[str]>, re: *_pcre) -> match {
    type matchstate = {
        m: option::t<[str]>,
        re: *_pcre
    };

    impl of match for matchstate {
        fn matched() -> bool { option::is_some::<[str]>(self.m) }
        fn substring(index: uint) -> str {
            option::get::<[str]>(self.m)[index]
        }
        fn substrings() -> [str] {
            option::get::<[str]>(self.m)
        }
        fn named(name: str) -> str unsafe {
            let _re = self.re;
            let idx = str::as_buf(name, { |_name|
                _native::pcre_get_stringnumber(_re, _name) });
            ret option::get::<[str]>(self.m)[idx - (1 as c_int)];
        }
    }
    ret { m : m, re: re } as match;
}

fn mk_pcre(re: str) -> pcre unsafe {
    type pcrestate = {
        _re: *_pcre,
        _res: _pcre_res
    };

    impl of pcre for pcrestate {
        fn match(target: str) -> match unsafe {
            let oveclen = 30;
            let ovec = vec::init_elt_mut(oveclen as uint, 0i32);
            let ovecp = vec::unsafe::to_ptr(ovec);
            let re = self._re;
            let r = str::as_buf(target, { |_target|
                _native::pcre_exec(re, ptr::null(),
                                   _target, str::byte_len(target) as c_int,
                                   0 as c_int, 0 as c_int, ovecp,
                                   oveclen as c_int)
            });
            if r < 0i32 {
                ret mk_match(option::none, re);
            }
            let idx = 2;    // skip the whole-string match at the start
            let res : [str] = [];
            while idx < oveclen * 2 / 3 {
                let start = ovec[idx];
                let end = ovec[idx + 1];
                idx = idx + 2;
                if start != end && start >= 0i32 && end >= 0i32 {
                    vec::grow(res, 1u, str::slice(target, start as uint,
                                                  end as uint));
                }
            }
            ret mk_match(option::some(res), re);
        }
    }

    let errv = ptr::null();
    let erroff = 0 as c_int;
    let r = str::as_buf(re, { |_re|
        _native::pcre_compile(_re, 0 as c_int, ptr::addr_of(errv),
                              ptr::addr_of(erroff), ptr::null())
    });
    if r == ptr::null() {
        fail #fmt["pcre_compile() failed: %s", str::from_cstr(errv)];
    }
    ret { _re: r, _res: _pcre_res(r) } as pcre;
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_match_basic() {
        let r = mk_pcre("...");
        let m = r.match("abc");
        assert(m.matched());
        assert(vec::is_empty(m.substrings()));
    }

    #[test]
    fn test_match_fail() {
        let r = mk_pcre("....");
        let m = r.match("ab");
        assert(!m.matched());
    }

    #[test]
    fn test_substring() {
        let r = mk_pcre("(.)bcd(e.g)");
        let m = r.match("abcdefg");
        assert(m.matched());
        assert(m.substring(0u) == "a");
        assert(m.substring(1u) == "efg");
    }

    #[test]
    fn test_named() {
        let r = mk_pcre("(?<foo>..).(?<bar>..)");
        let m = r.match("abcde");
        assert(m.matched());
        assert(m.named("foo") == "ab");
        assert(m.named("bar") == "de");
    }
}
