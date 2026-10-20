#include "json.h"

#include <cstdlib>

namespace jk::json {
namespace {

constexpr int kMaxDepth = 64;

class Parser {
public:
    explicit Parser(std::string_view text) : s_(text) {}

    bool Document(Value* out) {
        Space();
        if (!Parse(out, 0)) return false;
        Space();
        return i_ == s_.size();
    }

private:
    void Space() {
        while (i_ < s_.size() && (s_[i_] == ' ' || s_[i_] == '\t' || s_[i_] == '\n' || s_[i_] == '\r')) ++i_;
    }
    bool Literal(std::string_view word) {
        if (s_.substr(i_, word.size()) != word) return false;
        i_ += word.size();
        return true;
    }

    bool Parse(Value* v, int depth) {
        if (depth > kMaxDepth || i_ >= s_.size()) return false;
        switch (s_[i_]) {
            case '{': return Object(v, depth);
            case '[': return Array(v, depth);
            case '"': v->type = Value::Type::kString; return String(&v->string);
            case 't': v->type = Value::Type::kBool; v->boolean = true; return Literal("true");
            case 'f': v->type = Value::Type::kBool; return Literal("false");
            case 'n': return Literal("null");
        }
        return Number(v);
    }

    bool Object(Value* v, int depth) {
        v->type = Value::Type::kObject;
        ++i_;
        Space();
        if (i_ < s_.size() && s_[i_] == '}') return ++i_, true;
        while (true) {
            Space();
            std::string key;
            if (i_ >= s_.size() || s_[i_] != '"' || !String(&key)) return false;
            Space();
            if (i_ >= s_.size() || s_[i_++] != ':') return false;
            Space();
            Value member;
            if (!Parse(&member, depth + 1)) return false;
            v->object[key] = std::move(member);
            Space();
            if (i_ >= s_.size()) return false;
            if (s_[i_] == ',') { ++i_; continue; }
            if (s_[i_] == '}') return ++i_, true;
            return false;
        }
    }

    bool Array(Value* v, int depth) {
        v->type = Value::Type::kArray;
        ++i_;
        Space();
        if (i_ < s_.size() && s_[i_] == ']') return ++i_, true;
        while (true) {
            Space();
            Value item;
            if (!Parse(&item, depth + 1)) return false;
            v->array.push_back(std::move(item));
            Space();
            if (i_ >= s_.size()) return false;
            if (s_[i_] == ',') { ++i_; continue; }
            if (s_[i_] == ']') return ++i_, true;
            return false;
        }
    }

    static void Append(std::string* out, unsigned cp) {
        if (cp < 0x80) {
            *out += static_cast<char>(cp);
        } else if (cp < 0x800) {
            *out += static_cast<char>(0xC0 | (cp >> 6));
            *out += static_cast<char>(0x80 | (cp & 0x3F));
        } else if (cp < 0x10000) {
            *out += static_cast<char>(0xE0 | (cp >> 12));
            *out += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
            *out += static_cast<char>(0x80 | (cp & 0x3F));
        } else {
            *out += static_cast<char>(0xF0 | (cp >> 18));
            *out += static_cast<char>(0x80 | ((cp >> 12) & 0x3F));
            *out += static_cast<char>(0x80 | ((cp >> 6) & 0x3F));
            *out += static_cast<char>(0x80 | (cp & 0x3F));
        }
    }

    bool Hex4(unsigned* out) {
        if (i_ + 4 > s_.size()) return false;
        unsigned v = 0;
        for (int k = 0; k < 4; ++k) {
            const char c = s_[i_++];
            v <<= 4;
            if (c >= '0' && c <= '9') v |= static_cast<unsigned>(c - '0');
            else if (c >= 'a' && c <= 'f') v |= static_cast<unsigned>(c - 'a' + 10);
            else if (c >= 'A' && c <= 'F') v |= static_cast<unsigned>(c - 'A' + 10);
            else return false;
        }
        *out = v;
        return true;
    }

    bool String(std::string* out) {
        ++i_;  // the opening quote
        while (i_ < s_.size()) {
            const char c = s_[i_++];
            if (c == '"') return true;
            if (static_cast<unsigned char>(c) < 0x20) return false;
            if (c != '\\') {
                *out += c;
                continue;
            }
            if (i_ >= s_.size()) return false;
            switch (s_[i_++]) {
                case '"': *out += '"'; break;
                case '\\': *out += '\\'; break;
                case '/': *out += '/'; break;
                case 'b': *out += '\b'; break;
                case 'f': *out += '\f'; break;
                case 'n': *out += '\n'; break;
                case 'r': *out += '\r'; break;
                case 't': *out += '\t'; break;
                case 'u': {
                    unsigned cp;
                    if (!Hex4(&cp)) return false;
                    if (cp >= 0xD800 && cp < 0xDC00) {  // a surrogate pair
                        unsigned low;
                        if (i_ + 2 > s_.size() || s_[i_] != '\\' || s_[i_ + 1] != 'u') return false;
                        i_ += 2;
                        if (!Hex4(&low) || low < 0xDC00 || low >= 0xE000) return false;
                        cp = 0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
                    } else if (cp >= 0xDC00 && cp < 0xE000) {
                        return false;
                    }
                    Append(out, cp);
                    break;
                }
                default: return false;
            }
        }
        return false;
    }

    bool Number(Value* v) {
        const size_t start = i_;
        while (i_ < s_.size() && (std::string_view("+-0123456789.eE").find(s_[i_]) != std::string_view::npos)) ++i_;
        if (i_ == start) return false;
        const std::string digits(s_.substr(start, i_ - start));
        char* end = nullptr;
        v->number = std::strtod(digits.c_str(), &end);
        v->type = Value::Type::kNumber;
        return end && *end == '\0';
    }

    std::string_view s_;
    size_t i_ = 0;
};

}  // namespace

const Value* Value::Get(const std::string& key) const {
    if (type != Type::kObject) return nullptr;
    const auto it = object.find(key);
    return it == object.end() ? nullptr : &it->second;
}

std::optional<Value> Parse(std::string_view text) {
    if (text.substr(0, 3) == "\xEF\xBB\xBF") text.remove_prefix(3);  // a UTF-8 BOM
    Value v;
    if (!Parser(text).Document(&v)) return std::nullopt;
    return v;
}

}  // namespace jk::json
