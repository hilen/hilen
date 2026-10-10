// The app of the load test, `build/ios/load-test.rs`, see "What iOS allows"
// in docs/hot-reload.md. Not a part of the engine library. It tells if a
// development build on a real device may load a library from the folder a
// hot build keeps its libraries in.
//
// The app carries the same tiny library 3 times, with no signature, with an
// ad hoc one and with the one of the development certificate. It copies each
// into its data folder, loads it from there and prints what the system said.

#import <UIKit/UIKit.h>
#import <dlfcn.h>

int main(int argc, char* argv[]) {
    @autoreleasepool {
        // The same folder the hot loader takes, see `useOwnDir` there.
#if TARGET_OS_TV
        NSSearchPathDirectory place = NSCachesDirectory;
#else
        NSSearchPathDirectory place = NSApplicationSupportDirectory;
#endif
        NSString* data = NSSearchPathForDirectoriesInDomains(place, NSUserDomainMask, YES).firstObject;
        NSString* dir = [data stringByAppendingPathComponent:@"load-test"];
        NSFileManager* files = NSFileManager.defaultManager;
        NSError* error = nil;
        if (![files createDirectoryAtPath:dir withIntermediateDirectories:YES attributes:nil error:&error]) {
            fprintf(stderr, "load test: no folder %s: %s\n", dir.UTF8String, error.description.UTF8String);
            return 1;
        }

        for (NSString* name in @[ @"none", @"adhoc", @"development" ]) {
            NSString* from = [NSBundle.mainBundle pathForResource:name ofType:@"bin" inDirectory:@"libs"];
            NSString* to = [dir stringByAppendingPathComponent:[name stringByAppendingString:@".dylib"]];
            [files removeItemAtPath:to error:nil];
            if (from == nil || ![files copyItemAtPath:from toPath:to error:&error]) {
                fprintf(stderr, "load test: %s: not copied: %s\n", name.UTF8String, error.description.UTF8String);
                continue;
            }
            void* library = dlopen(to.fileSystemRepresentation, RTLD_NOW | RTLD_LOCAL);
            fprintf(stderr, "load test: %s: %s\n", name.UTF8String, library != NULL ? "loaded" : dlerror());
        }
        fprintf(stderr, "load test: done\n");
    }
    return 0;
}
