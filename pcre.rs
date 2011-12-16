use std;

export pcre, mk_pcre, match;

type match = obj {
    fn matched() -> bool;
    fn substring(index: uint) -> str;
    fn substrings() -> [str];
    fn named(name: str) -> str;
};

type pcre = obj {
    fn match(target: str) -> match;
};

#[link_name = "pcre"]
native mod _native {
    type _pcre;
    type _pcre_extra;
    fn pcre_compile(pattern: str::sbuf, options: int, errptr: *str::sbuf,
                    erroffset: *int, tableptr: *u8) -> *_pcre;
    fn pcre_exec(re: *_pcre, extra: *_pcre_extra, subject: str::sbuf,
                 length: int, startoffset: int, options: int,
                 ovector: *i32, ovecsize: int) -> i32;
    fn pcre_get_stringnumber(re: *_pcre, name: *u8) -> int;
    fn pcre_refcount(re: *_pcre, adj: int) -> int;
}

resource _pcre_res(re: *_native::_pcre) {
    _native::pcre_refcount(re, -1);
}

fn mk_match(m: option::t<[str]>, re: *_native::_pcre) -> match {
    obj match(m: option::t<[str]>, re: *_native::_pcre) {
        fn matched() -> bool { option::is_some::<[str]>(m) }
        fn substring(index: uint) -> str {
            option::get::<[str]>(m)[index]
        }
        fn substrings() -> [str] {
            option::get::<[str]>(m)
        }
        fn named(name: str) -> str unsafe {
            let _re = re;
            let idx = str::as_buf(name, { |_name|
                _native::pcre_get_stringnumber(_re, _name) });
            ret option::get::<[str]>(m)[idx - 1];
        }
    }
    ret match(m, re);
}

fn mk_pcre(re: str) -> pcre unsafe {
    type pcrestate = {
        _re: *_native::_pcre,
        _res: _pcre_res
    };

    obj pcre(st: pcrestate) {
        fn match(target: str) -> match unsafe {
            let oveclen = 30;
            let ovec = vec::init_elt_mut::<i32>(0i32, oveclen as uint);
            let ovecp = vec::unsafe::to_ptr::<i32>(ovec);
            let re = st._re;
            let r = str::as_buf(target, { |_target|
                _native::pcre_exec(re, ptr::null(),
                                   _target, str::byte_len(target) as int,
                                   0, 0, ovecp, oveclen)
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
    let erroff = 0;
    let r = str::as_buf(re, { |_re|
        _native::pcre_compile(_re, 0, ptr::addr_of(errv), ptr::addr_of(erroff),
                              ptr::null())
    });
    if r == ptr::null() {
        fail #fmt["pcre_compile() failed: %s", str::str_from_cstr(errv)];
    }
    ret pcre({ _re: r, _res: _pcre_res(r) });
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
