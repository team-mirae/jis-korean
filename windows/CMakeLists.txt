# The JIS Korean TSF text input processor (jiskorean.dll) and its tests. tools/build.ps1 builds
# the engine with cargo first and passes it in as JK_ENGINE_LIB.
cmake_minimum_required(VERSION 3.28)
project(jiskorean LANGUAGES CXX RC)

option(JK_PREDICT "Build with the predictive layer (spec §10); the engine must match" ON)
option(JK_LEARNED "Embed the learned ranker's model (spec §10.13); off for the legacy engine" ON)
set(JK_ENGINE_LIB "" CACHE FILEPATH "jis_korean.lib, built by cargo for this architecture")
if(NOT EXISTS "${JK_ENGINE_LIB}")
    message(FATAL_ERROR "JK_ENGINE_LIB not found: '${JK_ENGINE_LIB}'. Build with tools/build.ps1.")
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
# The static CRT, as the engine (cargo -C target-feature=+crt-static): the DLL loads into every
# app, so it can't rely on a VC++ runtime being installed.
set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded)

set(JK_ENGINE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../engine")
set(JK_ICON_DIR "${CMAKE_CURRENT_SOURCE_DIR}/icons")

# Version from the engine's Cargo.toml (0.2.0-preview → 0,2,0,0).
file(STRINGS "${JK_ENGINE_DIR}/Cargo.toml" version_line REGEX "^version = ")
string(REGEX MATCH "\"([0-9]+)\\.([0-9]+)\\.([0-9]+)[^\"]*\"" _ "${version_line}")
set(JK_VERSION "${CMAKE_MATCH_0}")
string(REPLACE "\"" "" JK_VERSION "${JK_VERSION}")
set(JK_VERSION_NUMBERS "${CMAKE_MATCH_1},${CMAKE_MATCH_2},${CMAKE_MATCH_3},0")
configure_file(src/jiskorean.rc.in "${CMAKE_CURRENT_BINARY_DIR}/jiskorean.rc" @ONLY)

add_compile_options(/W4 /utf-8 /permissive- /EHsc)
add_compile_definitions(UNICODE _UNICODE NOMINMAX WIN32_LEAN_AND_MEAN _WIN32_WINNT=0x0A00)
if(JK_PREDICT)
    add_compile_definitions(JK_PREDICT)
    if(JK_LEARNED)
        add_compile_definitions(JK_LEARNED)
    endif()
endif()

# The engine and what Rust's standard library links against on Windows.
add_library(jk_engine INTERFACE)
target_include_directories(jk_engine INTERFACE "${JK_ENGINE_DIR}/include")
target_link_libraries(jk_engine INTERFACE "${JK_ENGINE_LIB}" kernel32 advapi32 ntdll userenv
                                          ws2_32 dbghelp bcrypt synchronization)

add_library(jk_keymap STATIC src/key_map.cpp)
target_include_directories(jk_keymap PUBLIC src)
target_link_libraries(jk_keymap PUBLIC jk_engine)

add_library(jiskorean SHARED
    src/candidate_window.cpp
    src/display_attribute.cpp
    src/dllmain.cpp
    src/edit_session.cpp
    src/json.cpp
    src/lang_bar.cpp
    src/log.cpp
    src/registration.cpp
    src/settings.cpp
    src/shared_data.cpp
    src/text_service.cpp
    src/training.cpp
    src/jiskorean.def
    "${CMAKE_CURRENT_BINARY_DIR}/jiskorean.rc")
target_include_directories(jiskorean PRIVATE src)
target_link_libraries(jiskorean PRIVATE jk_keymap jk_engine ole32 oleaut32 user32 gdi32 shell32)

enable_testing()
add_executable(jiskorean_tests tests/main.cpp src/json.cpp)
target_link_libraries(jiskorean_tests PRIVATE jk_keymap jk_engine)
target_compile_definitions(jiskorean_tests PRIVATE JK_DOCS_DIR="${CMAKE_CURRENT_SOURCE_DIR}/../docs")
add_test(NAME jiskorean_tests COMMAND jiskorean_tests)

# A window with a classic Edit and a RichEdit, for typing through the installed TIP by hand or
# by tools/type-test.ps1.
# Loads, activates and unloads the DLL in rounds: DLL_PROCESS_DETACH must give the shared data
# back. Needs TSF on a desktop, so it's run by hand (windows/README.md), not by CI.
add_executable(jk_unloadtest tests/unload.cpp)
target_link_libraries(jk_unloadtest PRIVATE ole32 psapi)

add_executable(jk_testpad WIN32 tests/testpad.cpp)
target_link_libraries(jk_testpad PRIVATE ole32 user32 shell32 imm32)
