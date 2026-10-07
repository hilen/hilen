// The loader of a hot build, see docs/hot-reload.md. Not a part of the engine
// library, the hot run script compiles it into an app of its own. It owns the process and
// `UIApplicationMain`. The whole program is a dynamic library. The loader
// starts it, and swaps it for a newer one while the app keeps running.
//
// `HILEN_HOT_DIR` is a folder with the libraries and a file `current` that
// holds the file name of the newest one.

#import <UIKit/UIKit.h>
#import <dlfcn.h>

typedef int (*StartFn)(void);
typedef void (*StopFn)(void);
typedef int (*StoppedFn)(void);

static const double kStopPollSeconds = 0.05;
static const int kStopPollTries = 100;

@interface HilenLoader : UIResponder <UIApplicationDelegate>
@end

@implementation HilenLoader {
    NSString* _dir;
    NSString* _name;
    void* _library;
    BOOL _reloading;
    dispatch_source_t _watch;
}

- (BOOL)application:(UIApplication*)application didFinishLaunchingWithOptions:(NSDictionary*)options {
    _dir = NSProcessInfo.processInfo.environment[@"HILEN_HOT_DIR"];
    if (_dir == nil) {
        NSLog(@"hilen loader: HILEN_HOT_DIR is not set");
        return YES;
    }

    NSString* name = [self newestName];
    void* library = [self load:name];
    if (library != NULL) {
        [self start:library name:name];
    }
    [self watch];
    return YES;
}

- (NSString*)newestName {
    NSString* pointer = [_dir stringByAppendingPathComponent:@"current"];
    NSString* text = [NSString stringWithContentsOfFile:pointer encoding:NSUTF8StringEncoding error:nil];
    return [text stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];
}

- (void*)load:(NSString*)name {
    if (name.length == 0) {
        NSLog(@"hilen loader: no library named in %@/current", _dir);
        return NULL;
    }
    NSString* path = [_dir stringByAppendingPathComponent:name];
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

- (void)start:(void*)library name:(NSString*)name {
    NSLog(@"hilen loader: starting %@", name);
    _library = library;
    _name = name;
    ((StartFn)dlsym(library, "hilen_start_app"))();
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
    NSString* name = [self newestName];
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
    [self startWhenStopped:library name:name tries:kStopPollTries];
}

- (void)startWhenStopped:(void*)library name:(NSString*)name tries:(int)tries {
    if (_library == NULL || ((StoppedFn)dlsym(_library, "hilen_stopped"))()) {
        [self start:library name:name];
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
                       [self startWhenStopped:library name:name tries:tries - 1];
                   });
}

@end

int main(int argc, char* argv[]) {
    return UIApplicationMain(argc, argv, nil, @"HilenLoader");
}
