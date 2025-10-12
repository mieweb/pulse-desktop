//
//  SCRecorder.m
//  ScreenCaptureKit wrapper implementation
//
//  Captures screen using ScreenCaptureKit and writes to MP4 using AVAssetWriter
//

#import <Foundation/Foundation.h>
#import <AVFoundation/AVFoundation.h>
#import <ScreenCaptureKit/ScreenCaptureKit.h>
#import <CoreMedia/CoreMedia.h>
#import <CoreVideo/CoreVideo.h>
#import "SCRecorder.h"

@interface SCRecorderImpl : NSObject <SCStreamOutput, SCStreamDelegate, AVCaptureAudioDataOutputSampleBufferDelegate>

@property (nonatomic, strong) SCStream *stream;
@property (nonatomic, strong) AVAssetWriter *assetWriter;
@property (nonatomic, strong) AVAssetWriterInput *videoInput;
@property (nonatomic, strong) AVAssetWriterInput *audioInput;
@property (nonatomic, strong) AVCaptureSession *audioSession;
@property (nonatomic, strong) NSString *outputPath;
@property (nonatomic, strong) NSString *lastError;
@property (nonatomic, assign) NSTimeInterval startTime;
@property (nonatomic, assign) CMTime firstFrameTime;
@property (nonatomic, assign) CMTime firstAudioTime;
@property (nonatomic, assign) BOOL hasFirstFrame;
@property (nonatomic, assign) BOOL hasFirstAudio;
@property (nonatomic, assign) BOOL isRecording;
@property (nonatomic, assign) BOOL captureAudio;
@property (nonatomic, assign) uint32_t fps;
@property (nonatomic, assign) uint32_t width;
@property (nonatomic, assign) uint32_t height;

- (instancetype)initWithConfig:(const char*)path
                         width:(uint32_t)w
                        height:(uint32_t)h
                           fps:(uint32_t)f
                       quality:(uint32_t)q
                     displayID:(uint32_t)displayID
                  captureAudio:(BOOL)captureAudio;
- (int32_t)start;
- (int32_t)stop;
- (double)duration;

@end

@implementation SCRecorderImpl

- (instancetype)initWithConfig:(const char*)path
                         width:(uint32_t)w
                        height:(uint32_t)h
                           fps:(uint32_t)f
                       quality:(uint32_t)q
                     displayID:(uint32_t)displayID
                  captureAudio:(BOOL)captureAudio {
    self = [super init];
    if (self) {
        _outputPath = [NSString stringWithUTF8String:path];
        _width = w;
        _height = h;
        _fps = f;
        _captureAudio = captureAudio;
        _isRecording = NO;
        _hasFirstFrame = NO;
        _hasFirstAudio = NO;
        _firstFrameTime = kCMTimeZero;
        _firstAudioTime = kCMTimeZero;
        _lastError = nil;
        
        // Initialize asset writer
        NSError *error = nil;
        NSURL *outputURL = [NSURL fileURLWithPath:_outputPath];
        _assetWriter = [[AVAssetWriter alloc] initWithURL:outputURL
                                                  fileType:AVFileTypeMPEG4
                                                     error:&error];
        if (error) {
            _lastError = [NSString stringWithFormat:@"Failed to create asset writer: %@", error.localizedDescription];
            return nil;
        }
        
        // Configure video settings for H.264
        NSDictionary *videoSettings = @{
            AVVideoCodecKey: AVVideoCodecTypeH264,
            AVVideoWidthKey: @(w),
            AVVideoHeightKey: @(h),
            AVVideoCompressionPropertiesKey: @{
                AVVideoAverageBitRateKey: @(w * h * 3 * f / 4), // Reasonable bitrate
                AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel,
                AVVideoMaxKeyFrameIntervalKey: @(f * 2), // Keyframe every 2 seconds
            }
        };
        
        _videoInput = [AVAssetWriterInput assetWriterInputWithMediaType:AVMediaTypeVideo
                                                         outputSettings:videoSettings];
        _videoInput.expectsMediaDataInRealTime = YES;
        
        if ([_assetWriter canAddInput:_videoInput]) {
            [_assetWriter addInput:_videoInput];
        } else {
            _lastError = @"Cannot add video input to asset writer";
            return nil;
        }
        
        // Add audio input if requested
        if (captureAudio) {
            NSDictionary *audioSettings = @{
                AVFormatIDKey: @(kAudioFormatMPEG4AAC),
                AVSampleRateKey: @(48000),
                AVNumberOfChannelsKey: @(1),  // Mono
                AVEncoderBitRateKey: @(128000)  // 128 kbps
            };
            
            _audioInput = [AVAssetWriterInput assetWriterInputWithMediaType:AVMediaTypeAudio
                                                             outputSettings:audioSettings];
            _audioInput.expectsMediaDataInRealTime = YES;
            
            if ([_assetWriter canAddInput:_audioInput]) {
                [_assetWriter addInput:_audioInput];
            } else {
                _lastError = @"Cannot add audio input to asset writer";
                return nil;
            }
        }
    }
    return self;
}

- (int32_t)start {
    if (_isRecording) {
        _lastError = @"Already recording";
        return -1;
    }
    
    // Get available content
    __block SCRecorderImpl *weakSelf = self;
    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    __block int32_t result = 0;
    
    [SCShareableContent getShareableContentWithCompletionHandler:^(SCShareableContent *content, NSError *error) {
        if (error) {
            weakSelf.lastError = [NSString stringWithFormat:@"Failed to get shareable content: %@", error.localizedDescription];
            result = -1;
            dispatch_semaphore_signal(semaphore);
            return;
        }
        
        // Get primary display
        SCDisplay *display = content.displays.firstObject;
        if (!display) {
            weakSelf.lastError = @"No displays found";
            result = -1;
            dispatch_semaphore_signal(semaphore);
            return;
        }
        
        // Create content filter for the display
        SCContentFilter *filter = [[SCContentFilter alloc] initWithDisplay:display
                                                            excludingWindows:@[]];
        
        // Configure stream
        SCStreamConfiguration *config = [[SCStreamConfiguration alloc] init];
        config.width = weakSelf.width;
        config.height = weakSelf.height;
        config.minimumFrameInterval = CMTimeMake(1, weakSelf.fps);
        config.queueDepth = 5;
        config.pixelFormat = kCVPixelFormatType_32BGRA;
        config.showsCursor = YES;
        
        // Create stream
        weakSelf.stream = [[SCStream alloc] initWithFilter:filter
                                             configuration:config
                                                  delegate:weakSelf];
        
        NSError *streamError = nil;
        [weakSelf.stream addStreamOutput:weakSelf
                                    type:SCStreamOutputTypeScreen
                      sampleHandlerQueue:dispatch_get_global_queue(DISPATCH_QUEUE_PRIORITY_HIGH, 0)
                                   error:&streamError];
        
        if (streamError) {
            weakSelf.lastError = [NSString stringWithFormat:@"Failed to add stream output: %@", streamError.localizedDescription];
            result = -1;
            dispatch_semaphore_signal(semaphore);
            return;
        }
        
        // Start asset writer
        if (![weakSelf.assetWriter startWriting]) {
            weakSelf.lastError = [NSString stringWithFormat:@"Failed to start writing: %@", weakSelf.assetWriter.error];
            result = -1;
            dispatch_semaphore_signal(semaphore);
            return;
        }
        [weakSelf.assetWriter startSessionAtSourceTime:kCMTimeZero];
        
        // Reset first frame tracking
        weakSelf.hasFirstFrame = NO;
        weakSelf.firstFrameTime = kCMTimeZero;
        weakSelf.hasFirstAudio = NO;
        weakSelf.firstAudioTime = kCMTimeZero;
        
        // Setup audio capture if requested
        if (weakSelf.captureAudio) {
            [weakSelf setupAudioCapture];
        }
        
        // Start capture
        [weakSelf.stream startCaptureWithCompletionHandler:^(NSError *error) {
            if (error) {
                weakSelf.lastError = [NSString stringWithFormat:@"Failed to start capture: %@", error.localizedDescription];
                result = -1;
            } else {
                weakSelf.isRecording = YES;
                weakSelf.startTime = [NSDate timeIntervalSinceReferenceDate];
                result = 0;
            }
            dispatch_semaphore_signal(semaphore);
        }];
    }];
    
    // Wait for completion
    dispatch_semaphore_wait(semaphore, DISPATCH_TIME_FOREVER);
    return result;
}

- (int32_t)stop {
    if (!_isRecording) {
        _lastError = @"Not recording";
        return -1;
    }
    
    __block SCRecorderImpl *weakSelf = self;
    dispatch_semaphore_t semaphore = dispatch_semaphore_create(0);
    __block int32_t result = 0;
    
    [_stream stopCaptureWithCompletionHandler:^(NSError *error) {
        if (error) {
            weakSelf.lastError = [NSString stringWithFormat:@"Failed to stop capture: %@", error.localizedDescription];
            result = -1;
        }
        
        // Finish writing
        [weakSelf.videoInput markAsFinished];
        if (weakSelf.audioInput) {
            [weakSelf.audioInput markAsFinished];
        }
        if (weakSelf.audioSession) {
            [weakSelf.audioSession stopRunning];
        }
        [weakSelf.assetWriter finishWritingWithCompletionHandler:^{
            if (weakSelf.assetWriter.status == AVAssetWriterStatusFailed) {
                weakSelf.lastError = [NSString stringWithFormat:@"Asset writer failed: %@", weakSelf.assetWriter.error];
                result = -1;
            }
            weakSelf.isRecording = NO;
            dispatch_semaphore_signal(semaphore);
        }];
    }];
    
    dispatch_semaphore_wait(semaphore, DISPATCH_TIME_FOREVER);
    return result;
}

- (double)duration {
    if (_isRecording) {
        return [NSDate timeIntervalSinceReferenceDate] - _startTime;
    }
    return 0.0;
}

- (void)setupAudioCapture {
    _audioSession = [[AVCaptureSession alloc] init];
    
    // Get default microphone
    AVCaptureDevice *audioDevice = [AVCaptureDevice defaultDeviceWithMediaType:AVMediaTypeAudio];
    if (!audioDevice) {
        NSLog(@"⚠️ No microphone found, continuing without audio");
        return;
    }
    
    NSError *error = nil;
    AVCaptureDeviceInput *audioInput = [AVCaptureDeviceInput deviceInputWithDevice:audioDevice error:&error];
    if (error || !audioInput) {
        NSLog(@"⚠️ Failed to create audio input: %@", error.localizedDescription);
        return;
    }
    
    if ([_audioSession canAddInput:audioInput]) {
        [_audioSession addInput:audioInput];
    } else {
        NSLog(@"⚠️ Cannot add audio input to session");
        return;
    }
    
    // Setup audio output
    AVCaptureAudioDataOutput *audioOutput = [[AVCaptureAudioDataOutput alloc] init];
    dispatch_queue_t audioQueue = dispatch_queue_create("com.pulse.audioQueue", DISPATCH_QUEUE_SERIAL);
    [audioOutput setSampleBufferDelegate:self queue:audioQueue];
    
    if ([_audioSession canAddOutput:audioOutput]) {
        [_audioSession addOutput:audioOutput];
    } else {
        NSLog(@"⚠️ Cannot add audio output to session");
        return;
    }
    
    // Start audio session
    [_audioSession startRunning];
    NSLog(@"🎤 Audio capture started");
}

// AVCaptureAudioDataOutputSampleBufferDelegate method
- (void)captureOutput:(AVCaptureOutput *)output didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer fromConnection:(AVCaptureConnection *)connection {
    if (!_isRecording || !_audioInput) return;
    
    if (_audioInput.readyForMoreMediaData) {
        // Get the original presentation timestamp
        CMTime originalTime = CMSampleBufferGetPresentationTimeStamp(sampleBuffer);
        
        // On first audio frame, record the start time offset
        if (!_hasFirstAudio) {
            _firstAudioTime = originalTime;
            _hasFirstAudio = YES;
        }
        
        // Calculate adjusted time (relative to first audio frame = zero)
        CMTime adjustedTime = CMTimeSubtract(originalTime, _firstAudioTime);
        
        // Create new sample buffer with adjusted timestamp
        CMSampleBufferRef adjustedBuffer = NULL;
        CMSampleTimingInfo timingInfo;
        timingInfo.presentationTimeStamp = adjustedTime;
        timingInfo.decodeTimeStamp = kCMTimeInvalid;
        timingInfo.duration = CMSampleBufferGetDuration(sampleBuffer);
        
        OSStatus status = CMSampleBufferCreateCopyWithNewTiming(
            kCFAllocatorDefault,
            sampleBuffer,
            1,
            &timingInfo,
            &adjustedBuffer
        );
        
        if (status == noErr && adjustedBuffer != NULL) {
            [_audioInput appendSampleBuffer:adjustedBuffer];
            CFRelease(adjustedBuffer);
        }
    }
}

// SCStreamOutput delegate method
- (void)stream:(SCStream *)stream didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer ofType:(SCStreamOutputType)type {
    if (type == SCStreamOutputTypeScreen && _isRecording) {
        if (_videoInput.readyForMoreMediaData) {
            // Get the original presentation timestamp
            CMTime originalTime = CMSampleBufferGetPresentationTimeStamp(sampleBuffer);
            
            // On first frame, record the start time offset
            if (!_hasFirstFrame) {
                _firstFrameTime = originalTime;
                _hasFirstFrame = YES;
            }
            
            // Calculate adjusted time (relative to first frame = zero)
            CMTime adjustedTime = CMTimeSubtract(originalTime, _firstFrameTime);
            
            // Create new sample buffer with adjusted timestamp
            CMSampleBufferRef adjustedBuffer = NULL;
            CMSampleTimingInfo timingInfo;
            timingInfo.presentationTimeStamp = adjustedTime;
            timingInfo.decodeTimeStamp = kCMTimeInvalid;
            timingInfo.duration = CMTimeMake(1, _fps);
            
            OSStatus status = CMSampleBufferCreateCopyWithNewTiming(
                kCFAllocatorDefault,
                sampleBuffer,
                1,
                &timingInfo,
                &adjustedBuffer
            );
            
            if (status == noErr && adjustedBuffer != NULL) {
                [_videoInput appendSampleBuffer:adjustedBuffer];
                CFRelease(adjustedBuffer);
            }
        }
    }
}

@end

// C API implementation

struct SCRecorder {
    void *impl; // Opaque pointer to SCRecorderImpl
};

SCRecorder* sc_recorder_create(
    const char* output_path,
    uint32_t width,
    uint32_t height,
    uint32_t fps,
    uint32_t quality,
    uint32_t display_id,
    bool capture_audio
) {
    @autoreleasepool {
        SCRecorderImpl *impl = [[SCRecorderImpl alloc] initWithConfig:output_path
                                                                 width:width
                                                                height:height
                                                                   fps:fps
                                                               quality:quality
                                                             displayID:display_id
                                                          captureAudio:capture_audio];
        if (!impl) {
            return NULL;
        }
        
        SCRecorder *recorder = (SCRecorder*)malloc(sizeof(SCRecorder));
        recorder->impl = (__bridge_retained void*)impl;
        return recorder;
    }
}

int32_t sc_recorder_start(SCRecorder* recorder) {
    @autoreleasepool {
        if (!recorder) return -1;
        SCRecorderImpl *impl = (__bridge SCRecorderImpl*)(recorder->impl);
        return [impl start];
    }
}

int32_t sc_recorder_stop(SCRecorder* recorder) {
    @autoreleasepool {
        if (!recorder) return -1;
        SCRecorderImpl *impl = (__bridge SCRecorderImpl*)(recorder->impl);
        return [impl stop];
    }
}

double sc_recorder_duration(SCRecorder* recorder) {
    @autoreleasepool {
        if (!recorder) return 0.0;
        SCRecorderImpl *impl = (__bridge SCRecorderImpl*)(recorder->impl);
        return [impl duration];
    }
}

void sc_recorder_free(SCRecorder* recorder) {
    @autoreleasepool {
        if (recorder) {
            SCRecorderImpl *impl = (__bridge_transfer SCRecorderImpl*)(recorder->impl);
            (void)impl; // Just to release it
            free(recorder);
        }
    }
}

void sc_recorder_set_callback(
    SCRecorder* recorder,
    SCRecorderCallback callback,
    void* user_data
) {
    // TODO: Implement callback system
    (void)recorder;
    (void)callback;
    (void)user_data;
}

const char* sc_recorder_last_error(SCRecorder* recorder) {
    @autoreleasepool {
        if (!recorder) return NULL;
        SCRecorderImpl *impl = (__bridge SCRecorderImpl*)(recorder->impl);
        if (impl.lastError) {
            return [impl.lastError UTF8String];
        }
        return NULL;
    }
}
