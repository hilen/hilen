// The loader of a hot build, see docs/hot-reload.md. Not a part of the engine
// library, the hot run script compiles it into an app of its own. It owns the process and
// `UIApplicationMain`. The whole program is a dynamic library. The loader
// starts it, and swaps it for a newer one while the app keeps running.
//
// `HILEN_HOT_DIR` is a folder with the libraries and a file `current`. Its
// first line is the file name of the library to run. A second line, when it
// is there, is the folder that holds the `assets` of that app, the engine
// reads it from `HILEN_HOT_ROOT`. The loader writes the name of the library
// it started into the file `started`.
//
// On a phone nothing sets `HILEN_HOT_DIR`, the folder is `hot` in the data
// of the loader, and the app that runs fills it over the network. The loader
// app can carry a library of its own, `Frameworks/default.dylib`, which runs
// when nothing was sent yet or the sent library does not start.

#import <UIKit/UIKit.h>
#import <dlfcn.h>
#import <sys/resource.h>

typedef int (*StartFn)(void);
typedef void (*StopFn)(void);
typedef int (*StoppedFn)(void);
typedef void (*FreeHeapFn)(void);

static const double kStopPollSeconds = 0.05;
static const int kStopPollTries = 100;
// A stopped library still has threads that end, and the system still lets go
// of its last objects. Its heap is freed only after this long.
static const double kHeapGraceSeconds = 5;
// The name the packed library goes by, in `started` too.
static NSString* const kDefaultName = @"default";
// This file is there while a sent library starts. A start that ends the
// process leaves it, and the next launch then runs the packed library.
static NSString* const kStartingFile = @"starting";

@interface HilenLoader : UIResponder <UIApplicationDelegate>
@end

@implementation HilenLoader {
    NSString* _dir;
    NSString* _name;
    void* _library;
    BOOL _reloading;
    // The stopped libraries whose heap is not freed yet, each with the time
    // of its stop.
    NSMutableArray<NSArray*>* _stopped;
    dispatch_source_t _watch;
}

- (BOOL)application:(UIApplication*)application didFinishLaunchingWithOptions:(NSDictionary*)options {
    _dir = NSProcessInfo.processInfo.environment[@"HILEN_HOT_DIR"];
    if (_dir == nil && ![self useOwnDir]) {
        return YES;
    }

    NSArray<NSString*>* newest = [self newest];
    NSString* name = newest[0];
    NSString* root = newest[1];
    if ([self lastStartFailed]) {
        NSLog(@"hilen loader: %@ ended the process at its start, it is not started again", name);
        name = @"";
    }
    void* library = name.length > 0 ? [self load:name] : NULL;
    if (library == NULL) {
        name = kDefaultName;
        root = @"";
        library = [self load:name];
    }
    if (library != NULL) {
        [self start:library name:name root:root];
    }
    [self watch];
    return YES;
}

// The hot folder of a phone, in the data of the loader. The engine reads the
// same variable to know where a sent library goes.
- (BOOL)useOwnDir {
    NSString* support =
        NSSearchPathForDirectoriesInDomains(NSApplicationSupportDirectory, NSUserDomainMask, YES).firstObject;
    _dir = [support stringByAppendingPathComponent:@"hot"];
    NSError* error = nil;
    if (![NSFileManager.defaultManager createDirectoryAtPath:_dir
                                 withIntermediateDirectories:YES
                                                  attributes:nil
                                                       error:&error]) {
        NSLog(@"hilen loader: no hot folder at %@: %@", _dir, error);
        return NO;
    }
    setenv("HILEN_HOT_DIR", _dir.fileSystemRepresentation, 1);
    return YES;
}

- (NSString*)startingFile {
    return [_dir stringByAppendingPathComponent:kStartingFile];
}

- (BOOL)lastStartFailed {
    return [NSFileManager.defaultManager fileExistsAtPath:[self startingFile]];
}

// The name of the library to run and the folder of its assets, each empty
// when the file does not have it.
- (NSArray<NSString*>*)newest {
    NSString* pointer = [_dir stringByAppendingPathComponent:@"current"];
    NSString* text = [NSString stringWithContentsOfFile:pointer encoding:NSUTF8StringEncoding error:nil];
    NSMutableArray<NSString*>* lines = [NSMutableArray array];
    for (NSString* line in [text componentsSeparatedByCharactersInSet:NSCharacterSet.newlineCharacterSet]) {
        [lines addObject:[line stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceCharacterSet]];
    }
    while (lines.count < 2) {
        [lines addObject:@""];
    }
    return lines;
}

- (void*)load:(NSString*)name {
    if (name.length == 0) {
        NSLog(@"hilen loader: no library named in %@/current", _dir);
        return NULL;
    }
    NSString* path = [_dir stringByAppendingPathComponent:name];
    if ([name isEqualToString:kDefaultName]) {
        path = [NSBundle.mainBundle.privateFrameworksPath stringByAppendingPathComponent:@"default.dylib"];
        if (![NSFileManager.defaultManager fileExistsAtPath:path]) {
            return NULL;
        }
    }
    void* library = dlopen(path.UTF8String, RTLD_NOW | RTLD_LOCAL);
    if (library == NULL) {
        NSLog(@"hilen loader: %s", dlerror());
        return NULL;
    }
    if (dlsym(library, "hilen_start_app") == NULL || dlsym(library, "hilen_stop") == NULL ||
        dlsym(library, "hilen_stopped") == NULL) {
        NSLog(@"hilen loader: %@ is not a hot build", name);
        return NULL;
    }
    return library;
}

- (void)start:(void*)library name:(NSString*)name root:(NSString*)root {
    NSLog(@"hilen loader: starting %@", name);
    _library = library;
    _name = name;
    if (root.length > 0) {
        setenv("HILEN_HOT_ROOT", root.fileSystemRepresentation, 1);
    } else {
        unsetenv("HILEN_HOT_ROOT");
    }
    BOOL sent = ![name isEqualToString:kDefaultName];
    if (sent) {
        [NSData.data writeToFile:[self startingFile] atomically:YES];
    }
    ((StartFn)dlsym(library, "hilen_start_app"))();
    if (sent) {
        [NSFileManager.defaultManager removeItemAtPath:[self startingFile] error:nil];
    }

    NSError* error = nil;
    NSString* started = [_dir stringByAppendingPathComponent:@"started"];
    if (![name writeToFile:started atomically:YES encoding:NSUTF8StringEncoding error:&error]) {
        NSLog(@"hilen loader: %@ is not written: %@", started, error);
    }
}

// A watch on the file would not follow a file that replaced it, so the
// folder is watched.
- (void)watch {
    int fd = open(_dir.fileSystemRepresentation, O_EVTONLY);
    if (fd < 0) {
        NSLog(@"hilen loader: cannot watch %@", _dir);
        return;
    }
    _watch = dispatch_source_create(DISPATCH_SOURCE_TYPE_VNODE, fd, DISPATCH_VNODE_WRITE, dispatch_get_main_queue());
    __weak HilenLoader* loader = self;
    dispatch_source_set_event_handler(_watch, ^{
        [loader reload];
    });
    dispatch_resume(_watch);
}

- (void)reload {
    if (_reloading) {
        return;
    }
    NSArray<NSString*>* newest = [self newest];
    NSString* name = newest[0];
    if (name.length == 0 || [name isEqualToString:_name]) {
        return;
    }

    // The new library is loaded before the old one stops, so a broken build
    // leaves the old one running.
    void* library = [self load:name];
    if (library == NULL) {
        return;
    }

    _reloading = YES;
    if (_library != NULL) {
        NSLog(@"hilen loader: stopping %@", _name);
        ((StopFn)dlsym(_library, "hilen_stop"))();
    }
    [self startWhenStopped:library name:name root:newest[1] tries:kStopPollTries];
}

// Every Rust allocation of a stopped library is in a heap of its own, see
// `hot/heap.rs` of the engine. A library that is never unloaded would keep
// all of it. The heap is freed at a later swap and not at the stop itself.
- (void)freeOldHeaps {
    if (_stopped == nil) {
        _stopped = [NSMutableArray array];
    }
    while (_stopped.count > 0) {
        NSArray* oldest = _stopped[0];
        if (-[oldest[1] timeIntervalSinceNow] < kHeapGraceSeconds) {
            return;
        }
        FreeHeapFn freeHeap = (FreeHeapFn)dlsym([oldest[0] pointerValue], "hilen_free_heap");
        if (freeHeap != NULL) {
            freeHeap();
        }
        [_stopped removeObjectAtIndex:0];
    }
}

- (void)startWhenStopped:(void*)library name:(NSString*)name root:(NSString*)root tries:(int)tries {
    if (_library == NULL || ((StoppedFn)dlsym(_library, "hilen_stopped"))()) {
        [self freeOldHeaps];
        if (_library != NULL) {
            [_stopped addObject:@[ [NSValue valueWithPointer:_library], NSDate.date ]];
        }
        [self start:library name:name root:root];
        _reloading = NO;
        // A build that came in meanwhile.
        [self reload];
        return;
    }
    if (tries == 0) {
        NSLog(@"hilen loader: %@ did not stop, %@ is not started", _name, name);
        _reloading = NO;
        return;
    }
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(kStopPollSeconds * NSEC_PER_SEC)),
                   dispatch_get_main_queue(), ^{
                       [self startWhenStopped:library name:name root:root tries:tries - 1];
                   });
}

@end

// A stopped library cannot give back every file it opened, some belong to
// statics of its dependencies. The usual limit of 256 open files would end
// the process after a few dozen swaps.
static void raiseFileLimit(void) {
    struct rlimit limit;
    if (getrlimit(RLIMIT_NOFILE, &limit) != 0) {
        return;
    }
    rlim_t wanted = limit.rlim_max < OPEN_MAX ? limit.rlim_max : OPEN_MAX;
    if (limit.rlim_cur < wanted) {
        limit.rlim_cur = wanted;
        if (setrlimit(RLIMIT_NOFILE, &limit) != 0) {
            NSLog(@"hilen loader: the open file limit stays at %llu", limit.rlim_cur);
        }
    }
}

int main(int argc, char* argv[]) {
    raiseFileLimit();
    return UIApplicationMain(argc, argv, nil, @"HilenLoader");
}
