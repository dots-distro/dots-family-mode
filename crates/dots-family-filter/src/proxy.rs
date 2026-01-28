                                            Ok(mut client_tls) => {
                                                let client_tls = Box::pin(tokio_openssl::SslStream<
                                                    hyper::upgrade::Upgraded,
                                                    TokioIo<hyper::upgrade::Upgraded>,
                                                >);