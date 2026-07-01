use reqwest::{Client, Method, RequestBuilder};

/// 通过 Edge Function 代理请求
pub struct ProxyHelper;

impl ProxyHelper {
    /// 将普通 URL 转换为通过 Edge Function 的代理 URL
    pub fn build_proxy_url(edge_function_url: &str, target_url: &str) -> String {
        // 移除 target_url 的 protocol
        let clean_target = target_url
            .trim_start_matches("https://")
            .trim_start_matches("http://");

        // 构建代理 URL（路径方式）
        if edge_function_url.ends_with("/") {
            format!("{}{}", edge_function_url, clean_target)
        } else {
            format!("{}/{}", edge_function_url, clean_target)
        }
    }

    /// 创建一个通过 Edge Function 的请求构建器
    pub fn proxy_request(
        client: &Client,
        method: Method,
        edge_function_url: &str,
        target_url: &str,
    ) -> RequestBuilder {
        let proxy_url = Self::build_proxy_url(edge_function_url, target_url);
        client.request(method, proxy_url)
    }
}

/// HTTP 客户端包装器，自动处理 Edge Function 代理
#[derive(Clone)]
pub struct ProxyClient {
    client: Client,
    edge_function_url: Option<String>,
}

#[allow(dead_code)]
impl ProxyClient {
    pub fn new(client: Client, edge_function_url: Option<String>) -> Self {
        ProxyClient {
            client,
            edge_function_url,
        }
    }

    /// GET 请求
    pub fn get(&self, url: &str) -> RequestBuilder {
        if let Some(ref edge_url) = self.edge_function_url {
            ProxyHelper::proxy_request(&self.client, Method::GET, edge_url, url)
        } else {
            self.client.get(url)
        }
    }

    /// POST 请求
    pub fn post(&self, url: &str) -> RequestBuilder {
        if let Some(ref edge_url) = self.edge_function_url {
            ProxyHelper::proxy_request(&self.client, Method::POST, edge_url, url)
        } else {
            self.client.post(url)
        }
    }

    /// PUT 请求
    pub fn put(&self, url: &str) -> RequestBuilder {
        if let Some(ref edge_url) = self.edge_function_url {
            ProxyHelper::proxy_request(&self.client, Method::PUT, edge_url, url)
        } else {
            self.client.put(url)
        }
    }

    /// DELETE 请求
    pub fn delete(&self, url: &str) -> RequestBuilder {
        if let Some(ref edge_url) = self.edge_function_url {
            ProxyHelper::proxy_request(&self.client, Method::DELETE, edge_url, url)
        } else {
            self.client.delete(url)
        }
    }

    /// HEAD 请求
    pub fn head(&self, url: &str) -> RequestBuilder {
        if let Some(ref edge_url) = self.edge_function_url {
            ProxyHelper::proxy_request(&self.client, Method::HEAD, edge_url, url)
        } else {
            self.client.head(url)
        }
    }

    /// 通用请求方法
    pub fn request(&self, method: Method, url: &str) -> RequestBuilder {
        if let Some(ref edge_url) = self.edge_function_url {
            ProxyHelper::proxy_request(&self.client, method, edge_url, url)
        } else {
            self.client.request(method, url)
        }
    }

    /// 获取原始客户端（用于特殊情况）
    pub fn inner(&self) -> &Client {
        &self.client
    }
}
