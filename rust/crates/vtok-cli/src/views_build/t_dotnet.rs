use super::run;

const BUILD: &str = "MSBuild version 17.8.3+195e7f5a3 for .NET
  Determining projects to restore...
  Restored /src/App/App.csproj (in 1.2 sec).
/src/App/Program.cs(10,13): warning CS8618: Non-nullable property 'A' must contain a non-null value [/src/App/App.csproj]
/src/App/Models.cs(5,9): warning CS8618: Non-nullable property 'B' must contain a non-null value [/src/App/App.csproj]
/src/App/Program.cs(12,5): error CS1002: ; expected [/src/App/App.csproj]
/src/App/Program.cs(12,5): error CS1002: ; expected [/src/App/App.csproj]

Build FAILED.

/src/App/Program.cs(10,13): warning CS8618: Non-nullable property 'A' must contain a non-null value [/src/App/App.csproj]
/src/App/Models.cs(5,9): warning CS8618: Non-nullable property 'B' must contain a non-null value [/src/App/App.csproj]
/src/App/Program.cs(12,5): error CS1002: ; expected [/src/App/App.csproj]
    2 Warning(s)
    1 Error(s)

Time Elapsed 00:00:03.21
";

#[test]
fn dotnet_build() {
    let o = run("dotnet build -c Release", BUILD);
    assert!(o.contains("/src/App/Program.cs:12:5 error CS1002: ; expected"), "{o}");
    assert_eq!(o.matches("CS1002").count(), 1);
    assert!(o.contains("warning CS8618 (x2)") && o.contains("Build FAILED.") && o.contains("1 Error(s)") && o.contains("Time Elapsed"));
    assert!(!o.contains("MSBuild version") && !o.contains("Determining"));
    assert_eq!(run("msbuild App.sln /m", BUILD), o);
}

const TEST: &str = "Test run for /src/T/bin/Debug/net8.0/T.dll (.NETCoreApp,Version=v8.0)
Microsoft (R) Test Execution Command Line Tool Version 17.8.0
Starting test execution, please wait...
A total of 1 test files matched the specified pattern.
  Passed T.Tests.One [3 ms]
  Passed T.Tests.Two [1 ms]
  Failed T.Tests.Three [12 ms]
  Error Message:
   Assert.Equal() Failure
Expected: 1
Actual:   2
  Stack Trace:
     at Xunit.Assert.Equal[T](T expected, T actual)
     at T.Tests.Three() in /src/T/Tests.cs:line 21

Failed!  - Failed:     1, Passed:     2, Skipped:     0, Total:     3, Duration: 20 ms - T.dll (net8.0)
";

#[test]
fn dotnet_test() {
    let o = run("dotnet test", TEST);
    assert!(o.contains("FAILED T.Tests.Three") && o.contains("Assert.Equal() Failure") && o.contains("Expected: 1"), "{o}");
    assert!(o.contains("Tests.cs:line 21") && o.contains("Failed!  - Failed:     1, Passed:     2"));
    assert!(!o.contains("T.Tests.One") && !o.contains("Microsoft (R)") && !o.contains("Xunit.Assert"));
}

#[test]
fn dotnet_restore_and_format() {
    let s = "  Determining projects to restore...\n  Restored /a/A.csproj (in 1 sec).\n  Restored /b/B.csproj (in 2 sec).\n/a/A.csproj : error NU1101: Unable to find package Foo. No packages exist with this id in source(s): nuget.org\n";
    let o = run("dotnet restore", s);
    assert!(o.contains("NU1101") && o.contains("(2 restored, 0 built, 0 formatted)"));
    let f = "Formatted code file '/a/One.cs'.\nFormatted code file '/a/Two.cs'.\n/a/One.cs(1,1): warning WHITESPACE: Fix whitespace formatting. Replace 4 characters with ' '. [/a/A.csproj]\n";
    let o = run("dotnet format", f);
    assert!(o.contains("warning WHITESPACE") && o.contains("2 formatted") && !o.contains("Formatted code file"));
}
