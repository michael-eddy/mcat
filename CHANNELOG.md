## V0.1.2
#### concat
* start by collecting all the paths from the input.
    - if a dir is given ask for the files in there and add them.
    - url is broken down and saved the tempfile for lifetime.
    - normal path is added normal
* figure out smartly what is the main format (text / images / videos)
    - text combine together like the convert dir (make md and html pass as is.)
    - images concat vert (default) or hori
    - videos concat in time. (if ffmpeg lets you)
* if the main formats collide bail and say cannot collide them.
    - don't attempt to make text / md / html into images and then concat, leave them alone.
    - meaning it doesn't matter what the to is.
#### flags
- add the scale flag
- use spx and sc flags i already have..

## V0.1.1
now accepts multi input:
mcat file.docx file.pptx file.odt ..

## V0.1.0
First Release
