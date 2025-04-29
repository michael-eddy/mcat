## V0.1.52
* auto download is now an option through the flags --fetch-chormium, --fetch--ffmpeg. and also adding --fetch-clean to remove after them.

## V0.1.51
* fixed issue with zombie process of chromium
* removed the --raw flag (chromium sandbox should suffice)

## V0.1.5
* now says when a path doesn't exists instead of saying Failed Reading
* adding zoom, x, y in the inline-options (--inline-options "")

## V0.1.4
now closing kitty animations when interrupted mid way

## V0.1.3
removes feature that requires native-tls (for cross compile)

## V0.1.2
#### new features  
* concatenate images (vertical or horizontal)  
* concatenate videos (time based, must be same format)  
* scale image while maintaining center via --inline-options "scale=<f32>"
#### improved  
* text based concatenate

## V0.1.1
now accepts multi input:
mcat file.docx file.pptx file.odt ..

## V0.1.0
First Release
