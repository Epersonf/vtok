use super::{ls, run, view};

const CTEST: &str = "Test project /src/build
    Start 1: unit_a
1/3 Test #1: unit_a ...........................   Passed    0.01 sec
    Start 2: unit_b
2/3 Test #2: unit_b ...........................***Failed    0.02 sec
Expected 4 but got 5
at b_test.cpp:33
    Start 3: unit_c
3/3 Test #3: unit_c ...........................   Passed    0.01 sec

67% tests passed, 1 tests failed out of 3

Total Test time (real) =   0.05 sec

The following tests FAILED:
\t  2 - unit_b (Failed)
Errors while running CTest
";

#[test]
fn ctest_keeps_failures() {
    let o = run("ctest --output-on-failure", CTEST);
    assert!(o.contains("FAIL unit_b (Failed)") && o.contains("Expected 4 but got 5") && o.contains("b_test.cpp:33"), "{o}");
    assert!(o.contains("67% tests passed, 1 tests failed out of 3") && o.contains("2 - unit_b (Failed)") && o.contains("(2 passed)"));
    assert!(!o.contains("unit_a") && !o.contains("Start 1"));
}

const MAKE: &str = "make[1]: Entering directory '/src/build'
[ 10%] Building CXX object CMakeFiles/app.dir/a.cpp.o
In file included from /src/a.cpp:1:
/src/a.cpp:10:5: warning: unused variable 'x' [-Wunused-variable]
   10 |     int x;
      |         ^
/src/b.cpp:3:5: warning: unused variable 'y' [-Wunused-variable]
/src/b.cpp:7:9: error: 'foo' was not declared in this scope
    7 |     foo();
      |     ^~~
/src/b.cpp:7:9: note: suggested alternative: 'for'
[ 50%] Linking CXX executable app
/usr/bin/ld: a.o: in function `main': a.cpp:(.text+0x5): undefined reference to `helper()'
collect2: error: ld returned 1 exit status
make[2]: *** [CMakeFiles/app.dir/build.make:100: app] Error 1
make[1]: Leaving directory '/src/build'
make: *** [Makefile:90: all] Error 2
";

#[test]
fn make_errors() {
    let o = run("make -j4", MAKE);
    assert!(o.contains("/src/b.cpp:7:9: error: 'foo' was not declared in this scope"), "{o}");
    assert!(o.contains("undefined reference to `helper()'") && o.contains("collect2: error") && o.contains("make: *** [Makefile:90"));
    assert!(o.contains("@ /src/a.cpp:10:5") && o.contains("build FAILED: 1 compile errors, 2 warnings"));
    assert!(!o.contains("Building CXX") && !o.contains("Entering directory") && !o.contains("suggested alternative") && !o.contains("int x;"));
    assert_eq!(run("cd build && cmake --build .", MAKE), o);
}

#[test]
fn make_ok_and_cmake_error() {
    let ok = "[ 33%] Building C object a.o\n[ 66%] Linking C executable app\n[100%] Built target app\n[100%] Built target lib\n";
    assert!(run("make", ok).contains("build ok (0 warnings, 2 targets)"));
    let cm = "-- The C compiler identification is GNU 12\n-- Detecting C compile features\nCMake Error at CMakeLists.txt:12 (find_package):\n  Could not find package Foo\n\n-- Configuring incomplete, errors occurred!\n";
    let o = run("cmake -S . -B build", cm);
    assert!(o.contains("CMake Error at CMakeLists.txt:12") && o.contains("Could not find package Foo") && !o.contains("compiler identification"));
}

#[test]
fn non_matching_is_none() {
    assert!(view("make", &ls("hello")).is_none());
    assert!(view("ctest", &ls("hello")).is_none());
}
