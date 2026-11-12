// A thin bridge to macOS's Contacts framework: it asks the system for permission, lists the
// address books (containers) of every account set up in System Settings, and hands their contacts
// back as vCard text, which the app already knows how to read. Read-only: it never changes a
// contact. Compiled by app/build.rs on macOS only.
//
// Each function returns a malloc'd C string the caller frees with hco_contacts_free, or NULL on
// failure. Strings are UTF-8. Built with ARC.

#import <Contacts/Contacts.h>
#import <Foundation/Foundation.h>
#include <dispatch/dispatch.h>
#include <stdlib.h>
#include <string.h>

static char *copy_string(NSString *text) {
    if (text == nil) {
        return NULL;
    }
    const char *utf8 = [text UTF8String];
    if (utf8 == NULL) {
        return NULL;
    }
    return strdup(utf8);
}

void hco_contacts_free(char *pointer) {
    free(pointer);
}

// Asks for permission if it has not been settled yet, and waits for the answer. Returns 1 when the
// app may read contacts, 0 when it may not.
int hco_contacts_access(void) {
    CNAuthorizationStatus status = [CNContactStore authorizationStatusForEntityType:CNEntityTypeContacts];
    if (status == CNAuthorizationStatusAuthorized) {
        return 1;
    }
    if (status == CNAuthorizationStatusDenied || status == CNAuthorizationStatusRestricted) {
        return 0;
    }
    __block int granted = 0;
    dispatch_semaphore_t wait = dispatch_semaphore_create(0);
    CNContactStore *store = [[CNContactStore alloc] init];
    [store requestAccessForEntityType:CNEntityTypeContacts
                    completionHandler:^(BOOL ok, NSError *_Nullable error) {
                        (void)error;
                        granted = ok ? 1 : 0;
                        dispatch_semaphore_signal(wait);
                    }];
    dispatch_semaphore_wait(wait, DISPATCH_TIME_FOREVER);
    return granted;
}

// The address books, one per line as "identifier\tname". NULL on error.
char *hco_contacts_containers(void) {
    @autoreleasepool {
        CNContactStore *store = [[CNContactStore alloc] init];
        NSError *error = nil;
        NSArray<CNContainer *> *containers = [store containersMatchingPredicate:nil error:&error];
        if (containers == nil) {
            return NULL;
        }
        NSMutableString *out = [NSMutableString string];
        for (CNContainer *container in containers) {
            NSString *name = container.name.length > 0 ? container.name : @"Contacts";
            [out appendFormat:@"%@\t%@\n", container.identifier, name];
        }
        return copy_string(out);
    }
}

// Every contact of one address book as vCard text (several cards, back to back). An empty or NULL
// identifier reads all contacts. NULL on error.
char *hco_contacts_vcards(const char *container_id) {
    @autoreleasepool {
        CNContactStore *store = [[CNContactStore alloc] init];
        NSArray<id<CNKeyDescriptor>> *keys = @[ [CNContactVCardSerialization descriptorForRequiredKeys] ];
        CNContactFetchRequest *request = [[CNContactFetchRequest alloc] initWithKeysToFetch:keys];
        if (container_id != NULL && container_id[0] != '\0') {
            NSString *identifier = [NSString stringWithUTF8String:container_id];
            request.predicate = [CNContact predicateForContactsInContainerWithIdentifier:identifier];
        }
        NSMutableArray<CNContact *> *contacts = [NSMutableArray array];
        NSError *error = nil;
        BOOL ok = [store enumerateContactsWithFetchRequest:request
                                                     error:&error
                                                usingBlock:^(CNContact *contact, BOOL *stop) {
                                                    (void)stop;
                                                    [contacts addObject:contact];
                                                }];
        if (!ok) {
            return NULL;
        }
        NSData *data = [CNContactVCardSerialization dataWithContacts:contacts error:&error];
        if (data == nil) {
            return NULL;
        }
        NSString *text = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
        return copy_string(text);
    }
}
