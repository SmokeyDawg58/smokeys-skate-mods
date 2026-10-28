using System;
using System.IO;
namespace SkateShared {
 static class FallbackUpdaterStub {
  static int Main(string[] args) {
   File.WriteAllText(Path.Combine(AppDomain.CurrentDomain.BaseDirectory,"launch-called.txt"),String.Join(" ",args));
   return 0;
  }
 }
}
