// The 2 functions the engine takes from the shell of an app, `hilen.h` of the
// mobile template. A hot build is a dynamic library with no such shell, see
// docs/hot-reload.md, so it carries them itself. Only the `hot` feature
// compiles this file.

#import <Foundation/Foundation.h>
#import <UIKit/UIKit.h>

void hilen_ios_show_alert(const char* message) {
    UIAlertController* alert =
        [UIAlertController alertControllerWithTitle:nil
                                            message:[NSString stringWithUTF8String:message]
                                     preferredStyle:UIAlertControllerStyleAlert];
    [alert addAction:[UIAlertAction actionWithTitle:@"OK" style:UIAlertActionStyleDefault handler:nil]];

    UIViewController* controller = UIApplication.sharedApplication.keyWindow.rootViewController;
    [controller presentViewController:alert animated:YES completion:nil];
}

const char* hilen_ios_get_icloud_storage_path(void) {
    NSURL* container = [NSFileManager.defaultManager URLForUbiquityContainerIdentifier:nil];
    if (container == nil) {
        return NULL;
    }
    return container.absoluteString.UTF8String;
}
