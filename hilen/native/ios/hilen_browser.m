//
//  hilen_browser.m
//  Hilen
//
//  A web page in a browser sheet over the app, SFSafariViewController. The
//  login opens its page here, so the user never leaves the app. It is the
//  system browser, Google allows a login in it and refuses one in a plain
//  web view.
//
//  The engine closes the sheet itself when the page has done its job. A
//  close by the user, with Done or with a swipe, goes to the engine through
//  `closed`.
//
//  The build script of the engine compiles this file into the engine library,
//  so this side and `browser_sheet.rs` are always the same version.
//

#import <SafariServices/SafariServices.h>
#import <UIKit/UIKit.h>

typedef void (*HilenBrowserClosed)(void);

@interface HilenBrowserBridge : NSObject <SFSafariViewControllerDelegate>
@end

// The sheet that is up now, nil with none.
static SFSafariViewController* hilen_browser = nil;
static HilenBrowserBridge* hilen_browser_bridge = nil;
static HilenBrowserClosed hilen_browser_closed = NULL;

@implementation HilenBrowserBridge

// Only a close by the user comes here, a dismiss from code does not.
- (void)safariViewControllerDidFinish:(SFSafariViewController*)controller {
    if (controller != hilen_browser) return;
    hilen_browser = nil;
    if (hilen_browser_closed) hilen_browser_closed();
}

@end

// The controller on top, a sheet can only go over that one.
static UIViewController* hilen_browser_host(void) {
    UIViewController* top = UIApplication.sharedApplication.keyWindow.rootViewController;
    while (top.presentedViewController && !top.presentedViewController.isBeingDismissed) {
        top = top.presentedViewController;
    }
    return top;
}

void hilen_browser_close(void) {
    if (!hilen_browser) return;
    SFSafariViewController* browser = hilen_browser;
    hilen_browser = nil;
    [browser.presentingViewController dismissViewControllerAnimated:YES completion:nil];
}

// The answer is false when no sheet opened. A sheet that is still up is
// closed first, with no word to the engine.
bool hilen_browser_open(const char* address, HilenBrowserClosed closed) {
    NSURL* url = [NSURL URLWithString:@(address)];
    NSString* scheme = url.scheme.lowercaseString;
    // The controller throws on any other scheme.
    if (![scheme isEqualToString:@"http"] && ![scheme isEqualToString:@"https"]) return false;

    hilen_browser_close();

    UIViewController* host = hilen_browser_host();
    if (!host) return false;

    if (!hilen_browser_bridge) hilen_browser_bridge = [HilenBrowserBridge new];
    hilen_browser_closed = closed;

    hilen_browser = [[SFSafariViewController alloc] initWithURL:url];
    hilen_browser.delegate = hilen_browser_bridge;
    [host presentViewController:hilen_browser animated:YES completion:nil];
    return true;
}
