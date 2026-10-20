// Display attributes: how apps underline the unconfirmed text.
#pragma once

#include <msctf.h>

namespace jk {

IEnumTfDisplayAttributeInfo* NewDisplayAttributeEnum();
ITfDisplayAttributeInfo* NewDisplayAttributeInfo(REFGUID guid);  // nullptr if not ours

}  // namespace jk
