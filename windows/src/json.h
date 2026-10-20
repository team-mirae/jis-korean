// A small JSON reader for training files (docs/training-contract.md): enough to read objects,
// arrays, strings, numbers, booleans and null. Never throws; bad input gives std::nullopt.
#pragma once

#include <map>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace jk::json {

struct Value {
    enum class Type { kNull, kBool, kNumber, kString, kArray, kObject } type = Type::kNull;
    bool boolean = false;
    double number = 0;
    std::string string;  // UTF-8
    std::vector<Value> array;
    std::map<std::string, Value> object;

    // The member `key` of an object, or nullptr.
    const Value* Get(const std::string& key) const;
    const std::string* String() const { return type == Type::kString ? &string : nullptr; }
    std::optional<double> Number() const {
        return type == Type::kNumber ? std::optional<double>(number) : std::nullopt;
    }
};

std::optional<Value> Parse(std::string_view text);

}  // namespace jk::json
